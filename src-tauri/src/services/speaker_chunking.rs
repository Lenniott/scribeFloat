//! Speaker-aware PCM cut points for Whisper chunking (pitch + loudness + silence).
//! Marks *where* the voice likely changed — identification is downstream.

use crate::services::audio::WHISPER_SAMPLE_RATE;
use crate::services::model::MIN_PCM_SAMPLES_16K;
use crate::services::output::hallucination::pcm_rms;
use serde::{Deserialize, Serialize};

pub const CHUNKING_SAMPLE_RATE: u32 = WHISPER_SAMPLE_RATE;

const FMIN: f64 = 65.0;
const FMAX: f64 = 400.0;
const FRAME_LENGTH: usize = 2048;
const HOP_LENGTH: usize = 256;
const WINDOW_SEC: f64 = 1.0;
const PITCH_JUMP_SEMITONES: f64 = 4.0;
const LOUDNESS_JUMP_DB: f64 = 6.0;
const MIN_SILENCE_GAP_S: f64 = 0.3;
const TOP_DB: f64 = 25.0;
const MIN_DURATION_FOR_CUTS_S: f64 = 5.0;
const SNAP_RADIUS_S: f64 = 0.3;
const MAX_CHUNK_S: f64 = 20.0;
const F0_FLOOR_MULT: f64 = 1.15;
const SEMITONE_REF_HZ: f64 = 100.0;
const EVAL_STEP_S: f64 = 0.1;
const MIN_CUT_SPACING_S: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutEvidence {
    PitchJump,
    LoudnessJump,
    SilenceGap,
    MaxSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerCut {
    pub time_s: f32,
    pub evidence: CutEvidence,
    pub strength: f32,
}

struct PitchTrack {
    times: Vec<f64>,
    semitones: Vec<f64>,
    voiced: Vec<bool>,
    rms: Vec<f32>,
}

/// Find speaker handover cut points in mono PCM.
pub fn find_cuts(pcm: &[f32], sample_rate: u32, enabled: bool) -> Vec<SpeakerCut> {
    debug_assert_eq!(sample_rate, CHUNKING_SAMPLE_RATE);
    if !enabled || pcm.is_empty() {
        return Vec::new();
    }
    let duration_s = pcm.len() as f64 / sample_rate as f64;
    tracing::info!(
        duration_s = format!("{duration_s:.1}"),
        "speaker chunking analysis"
    );
    if duration_s < MIN_DURATION_FOR_CUTS_S {
        return Vec::new();
    }

    let Some(track) = build_track(pcm, sample_rate) else {
        return Vec::new();
    };

    let mut cuts = Vec::new();
    cuts.extend(detect_jumps_pitch(&track));
    cuts.extend(detect_jumps_loudness(&track));
    cuts.extend(silence_cuts(pcm, sample_rate));

    cuts = union_cuts(cuts);
    cuts = cuts
        .into_iter()
        .map(|mut c| {
            c.time_s = snap_to_quietest(pcm, sample_rate, c.time_s);
            c
        })
        .collect();
    cuts = resolve_close_cuts(cuts, sample_rate);
    cuts = insert_max_span_cuts(cuts, duration_s);
    cuts.sort_by(|a, b| {
        a.time_s
            .partial_cmp(&b.time_s)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    cuts.dedup_by(|a, b| (a.time_s - b.time_s).abs() < 0.05);

    tracing::info!(
        cut_count = cuts.len(),
        duration_s = format!("{duration_s:.1}"),
        "speaker chunking cuts"
    );
    for cut in &cuts {
        tracing::info!(
            time_s = cut.time_s,
            evidence = ?cut.evidence,
            strength = cut.strength,
            "speaker cut"
        );
    }

    cuts
}

/// Split PCM at cut times. Returns `(offset_ms, owned chunk)` covering the full buffer.
pub fn split_pcm_owned(pcm: &[f32], cuts: &[SpeakerCut], sample_rate: u32) -> Vec<(u64, Vec<f32>)> {
    if cuts.is_empty() {
        return vec![(0, pcm.to_vec())];
    }
    let mut boundaries: Vec<usize> = cuts
        .iter()
        .map(|c| {
            let sample = (f64::from(c.time_s) * sample_rate as f64).round() as usize;
            sample.min(pcm.len())
        })
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut raw: Vec<(u64, Vec<f32>)> = Vec::new();
    let mut start = 0usize;
    for &end in &boundaries {
        if end > start {
            let offset_ms = (start as u64 * 1000) / sample_rate as u64;
            raw.push((offset_ms, pcm[start..end].to_vec()));
        }
        start = end;
    }
    if start < pcm.len() {
        let offset_ms = (start as u64 * 1000) / sample_rate as u64;
        raw.push((offset_ms, pcm[start..].to_vec()));
    }
    merge_too_short_chunks(raw, sample_rate)
}

fn merge_too_short_chunks(chunks: Vec<(u64, Vec<f32>)>, sample_rate: u32) -> Vec<(u64, Vec<f32>)> {
    if chunks.is_empty() {
        return chunks;
    }
    let min_samples = MIN_PCM_SAMPLES_16K;
    let mut merged: Vec<(u64, Vec<f32>)> = Vec::new();
    for (offset_ms, mut data) in chunks {
        if let Some((_, last)) = merged.last_mut() {
            if last.len() < min_samples || data.len() < min_samples {
                last.append(&mut data);
                continue;
            }
        }
        merged.push((offset_ms, data));
    }
    if merged.len() > 1 {
        let last_len = merged.last().map(|(_, d)| d.len()).unwrap_or(0);
        if last_len < min_samples {
            let (_, tail) = merged.pop().expect("last");
            if let Some((_, prev)) = merged.last_mut() {
                prev.extend(tail);
            } else {
                merged.push((0, tail));
            }
        }
    }
    let _ = sample_rate;
    merged
}

fn build_track(pcm: &[f32], sample_rate: u32) -> Option<PitchTrack> {
    let started = std::time::Instant::now();
    let rms = frame_rms(pcm, FRAME_LENGTH, HOP_LENGTH);
    if rms.is_empty() {
        return None;
    }
    let times: Vec<f64> = (0..rms.len())
        .map(|i| i as f64 * HOP_LENGTH as f64 / sample_rate as f64)
        .collect();
    let peak_rms = rms.iter().copied().fold(0.0f32, f32::max);
    let speech_floor = (peak_rms * 0.02).max(1e-5);
    let voiced: Vec<bool> = rms.iter().map(|&r| r >= speech_floor).collect();

    let mut semitones = vec![f64::NAN; rms.len()];
    let dt = if times.len() > 1 {
        times[1] - times[0]
    } else {
        HOP_LENGTH as f64 / sample_rate as f64
    };
    let pitch_step = (EVAL_STEP_S / dt).round().max(1.0) as usize;
    for i in (0..rms.len()).step_by(pitch_step) {
        if !voiced[i] {
            continue;
        }
        let start = i * HOP_LENGTH;
        let end = (start + FRAME_LENGTH).min(pcm.len());
        if end <= start {
            continue;
        }
        let frame = &pcm[start..end];
        if let Some(hz) = estimate_f0_yin(frame, sample_rate) {
            semitones[i] = hz_to_semitones(hz);
        }
    }

    tracing::info!(
        elapsed_ms = started.elapsed().as_millis(),
        frames = rms.len(),
        "speaker chunking pitch track"
    );

    Some(PitchTrack {
        times,
        semitones,
        voiced,
        rms,
    })
}

fn hz_to_semitones(hz: f64) -> f64 {
    let floor = FMIN * F0_FLOOR_MULT;
    if hz.is_nan() || hz < floor {
        f64::NAN
    } else {
        12.0 * (hz / SEMITONE_REF_HZ).log2()
    }
}

/// YIN-style fundamental frequency (Cheveigne & Kawahara) — faster than pYIN, good on speech.
fn estimate_f0_yin(frame: &[f32], sample_rate: u32) -> Option<f64> {
    if frame.len() < FRAME_LENGTH / 2 {
        return None;
    }
    let tau_min = (sample_rate as f64 / FMAX).ceil() as usize;
    let tau_max = (sample_rate as f64 / FMIN).floor() as usize;
    let tau_max = tau_max.min(frame.len() / 2);
    if tau_max <= tau_min {
        return None;
    }

    let mut d = vec![0.0f64; tau_max + 1];
    for tau in 1..=tau_max {
        let mut sum = 0.0f64;
        for i in 0..frame.len() - tau {
            let delta = f64::from(frame[i] - frame[i + tau]);
            sum += delta * delta;
        }
        d[tau] = sum;
    }

    let mut running_sum = 0.0f64;
    let threshold = 0.12;
    let mut tau = tau_min;
    while tau <= tau_max {
        running_sum += d[tau];
        let cmnd = if running_sum > 0.0 {
            d[tau] * tau as f64 / running_sum
        } else {
            1.0
        };
        if cmnd < threshold {
            while tau + 1 <= tau_max && d[tau + 1] < d[tau] {
                tau += 1;
            }
            let refined = if tau > 0 && tau < tau_max {
                let s0 = d[tau - 1];
                let s1 = d[tau];
                let s2 = d[tau + 1];
                let denom = s0 - 2.0 * s1 + s2;
                if denom.abs() > 1e-12 {
                    tau as f64 + (s0 - s2) / (2.0 * denom)
                } else {
                    tau as f64
                }
            } else {
                tau as f64
            };
            if refined >= tau_min as f64 {
                return Some(sample_rate as f64 / refined);
            }
            return None;
        }
        tau += 1;
    }
    None
}

fn frame_rms(pcm: &[f32], frame_length: usize, hop_length: usize) -> Vec<f32> {
    if pcm.is_empty() {
        return Vec::new();
    }
    let n_frames = pcm.len().saturating_sub(frame_length) / hop_length + 1;
    (0..n_frames)
        .map(|i| {
            let start = i * hop_length;
            let end = (start + frame_length).min(pcm.len());
            pcm_rms(&pcm[start..end])
        })
        .collect()
}

fn detect_jumps_pitch(track: &PitchTrack) -> Vec<SpeakerCut> {
    detect_voiced_jumps(track, PITCH_JUMP_SEMITONES, CutEvidence::PitchJump)
}

fn detect_jumps_loudness(track: &PitchTrack) -> Vec<SpeakerCut> {
    detect_voiced_jumps(track, LOUDNESS_JUMP_DB, CutEvidence::LoudnessJump)
}

fn detect_voiced_jumps(
    track: &PitchTrack,
    threshold: f64,
    evidence: CutEvidence,
) -> Vec<SpeakerCut> {
    if track.times.len() < 2 {
        return Vec::new();
    }
    let dt = track.times[1] - track.times[0];
    let step = (EVAL_STEP_S / dt).round().max(1.0) as usize;
    let win = (WINDOW_SEC / dt).round() as usize;
    let voiced_idx: Vec<usize> = track
        .voiced
        .iter()
        .enumerate()
        .filter_map(|(i, &speech)| {
            if evidence == CutEvidence::PitchJump {
                (!track.semitones[i].is_nan() && speech).then_some(i)
            } else {
                speech.then_some(i)
            }
        })
        .collect();

    let mut raw = Vec::new();
    for i in (0..track.semitones.len()).step_by(step) {
        let k = voiced_idx.partition_point(|&vi| vi < i);
        let b_idx = &voiced_idx[k.saturating_sub(win)..k];
        let a_idx = &voiced_idx[k..(k + win).min(voiced_idx.len())];
        if b_idx.len() < win / 4 || a_idx.len() < win / 4 {
            continue;
        }
        let (b_vals, a_vals) = if evidence == CutEvidence::LoudnessJump {
            (
                b_idx
                    .iter()
                    .map(|&i| f64::from(track.rms[i]))
                    .collect::<Vec<_>>(),
                a_idx
                    .iter()
                    .map(|&i| f64::from(track.rms[i]))
                    .collect::<Vec<_>>(),
            )
        } else {
            (
                b_idx
                    .iter()
                    .map(|&i| track.semitones[i])
                    .collect::<Vec<_>>(),
                a_idx
                    .iter()
                    .map(|&i| track.semitones[i])
                    .collect::<Vec<_>>(),
            )
        };
        let jump = if evidence == CutEvidence::LoudnessJump {
            let med_b = median_f64(&b_vals);
            let med_a = median_f64(&a_vals);
            if med_b <= 0.0 || med_a <= 0.0 {
                continue;
            }
            (20.0 * (med_a / med_b).log10()).abs()
        } else {
            (median_f64(&a_vals) - median_f64(&b_vals)).abs()
        };
        if jump > threshold {
            raw.push((track.times[i], jump));
        }
    }
    merge_raw_cuts(&raw, step, dt, evidence)
}

fn merge_raw_cuts(
    raw: &[(f64, f64)],
    step: usize,
    dt: f64,
    evidence: CutEvidence,
) -> Vec<SpeakerCut> {
    let mut merged: Vec<(f64, f64)> = Vec::new();
    let mut last_t: Option<f64> = None;
    for &(t, j) in raw {
        if let Some(lt) = last_t {
            if t - lt <= 2.0 * step as f64 * dt {
                if let Some(last) = merged.last_mut() {
                    if j > last.1 {
                        *last = (t, j);
                    }
                }
            } else {
                merged.push((t, j));
            }
        } else {
            merged.push((t, j));
        }
        last_t = Some(t);
    }
    merged
        .into_iter()
        .map(|(t, j)| SpeakerCut {
            time_s: t as f32,
            evidence,
            strength: j as f32,
        })
        .collect()
}

fn silence_cuts(pcm: &[f32], sample_rate: u32) -> Vec<SpeakerCut> {
    let rms_frames = frame_rms(pcm, FRAME_LENGTH, HOP_LENGTH);
    if rms_frames.is_empty() {
        return Vec::new();
    }
    let peak = rms_frames.iter().copied().fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return Vec::new();
    }
    let threshold = peak * 10f32.powf(-(TOP_DB as f32) / 20.0);
    let hop_s = HOP_LENGTH as f64 / sample_rate as f64;

    let mut speech_intervals: Vec<(f64, f64)> = Vec::new();
    let mut in_speech = false;
    let mut seg_start = 0.0;
    for (i, &r) in rms_frames.iter().enumerate() {
        let t = i as f64 * hop_s;
        let speech = r >= threshold;
        if speech && !in_speech {
            seg_start = t;
            in_speech = true;
        } else if !speech && in_speech {
            speech_intervals.push((seg_start, t));
            in_speech = false;
        }
    }
    if in_speech {
        let end = pcm.len() as f64 / sample_rate as f64;
        speech_intervals.push((seg_start, end));
    }

    let mut cuts = Vec::new();
    for window in speech_intervals.windows(2) {
        let (_, end) = window[0];
        let (start, _) = window[1];
        let gap = start - end;
        if gap >= MIN_SILENCE_GAP_S {
            cuts.push(SpeakerCut {
                time_s: ((end + start) / 2.0) as f32,
                evidence: CutEvidence::SilenceGap,
                strength: gap as f32,
            });
        }
    }
    cuts
}

fn union_cuts(mut cuts: Vec<SpeakerCut>) -> Vec<SpeakerCut> {
    cuts.sort_by(|a, b| {
        a.time_s
            .partial_cmp(&b.time_s)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut merged: Vec<SpeakerCut> = Vec::new();
    for cut in cuts {
        if let Some(last) = merged.last_mut() {
            if (last.time_s - cut.time_s).abs() < 0.15 {
                if cut.strength > last.strength {
                    *last = cut;
                }
                continue;
            }
        }
        merged.push(cut);
    }
    merged
}

fn snap_to_quietest(pcm: &[f32], sample_rate: u32, time_s: f32) -> f32 {
    let center = (f64::from(time_s) * sample_rate as f64).round() as isize;
    let radius = (SNAP_RADIUS_S * sample_rate as f64).round() as isize;
    let start = (center - radius).max(0) as usize;
    let end = (center + radius).min(pcm.len() as isize) as usize;
    if start >= end {
        return time_s;
    }
    let window = 256usize;
    let mut best_i = start;
    let mut best_rms = f32::MAX;
    let mut i = start;
    while i + window <= end {
        let r = pcm_rms(&pcm[i..i + window]);
        if r < best_rms {
            best_rms = r;
            best_i = i;
        }
        i += window / 2;
    }
    ((best_i as f64 + window as f64 / 2.0) / sample_rate as f64) as f32
}

fn resolve_close_cuts(mut cuts: Vec<SpeakerCut>, _sample_rate: u32) -> Vec<SpeakerCut> {
    let min_gap_s = MIN_CUT_SPACING_S;
    cuts.sort_by(|a, b| {
        a.time_s
            .partial_cmp(&b.time_s)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut resolved: Vec<SpeakerCut> = Vec::new();
    for cut in cuts {
        if let Some(last) = resolved.last() {
            let gap = f64::from(cut.time_s - last.time_s);
            if gap < min_gap_s {
                if cut.strength > last.strength {
                    resolved.pop();
                    resolved.push(cut);
                }
                continue;
            }
        }
        resolved.push(cut);
    }
    resolved
}

fn insert_max_span_cuts(mut cuts: Vec<SpeakerCut>, duration_s: f64) -> Vec<SpeakerCut> {
    loop {
        let mut boundaries: Vec<f64> = cuts.iter().map(|c| f64::from(c.time_s)).collect();
        boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut points = vec![0.0];
        points.extend(&boundaries);
        points.push(duration_s);

        let mut added = false;
        for window in points.windows(2) {
            let (a, b) = (window[0], window[1]);
            if b - a > MAX_CHUNK_S {
                let mid = (a + b) / 2.0;
                let duplicate = cuts
                    .iter()
                    .any(|c| (f64::from(c.time_s) - mid).abs() < 0.15);
                if !duplicate {
                    cuts.push(SpeakerCut {
                        time_s: mid as f32,
                        evidence: CutEvidence::MaxSpan,
                        strength: (b - a) as f32,
                    });
                    added = true;
                }
            }
        }
        if !added {
            break;
        }
    }
    cuts
}

fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Score cuts against known handover times (for tests and bench binary).
#[cfg_attr(not(any(test, feature = "bench")), allow(dead_code))]
pub fn score_cuts(cuts: &[SpeakerCut], truth_s: &[f64], tolerance_s: f64) -> CutScore {
    let cut_times: Vec<f64> = cuts.iter().map(|c| f64::from(c.time_s)).collect();
    let mut caught = 0usize;
    for &b in truth_s {
        if cut_times.iter().any(|c| (c - b).abs() <= tolerance_s) {
            caught += 1;
        }
    }
    let extra = cut_times
        .iter()
        .filter(|c| !truth_s.iter().any(|b| (*c - b).abs() <= tolerance_s))
        .count();
    CutScore {
        caught,
        total_truth: truth_s.len(),
        extra,
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(not(any(test, feature = "bench")), allow(dead_code))]
pub struct CutScore {
    pub caught: usize,
    pub total_truth: usize,
    pub extra: usize,
}

pub fn cuts_to_manifest(cuts: &[SpeakerCut]) -> Vec<crate::types::ManifestSpeakerCut> {
    cuts.iter()
        .map(|c| crate::types::ManifestSpeakerCut {
            time_s: c.time_s,
            evidence: evidence_label(c.evidence),
            strength: c.strength,
        })
        .collect()
}

fn evidence_label(evidence: CutEvidence) -> String {
    match evidence {
        CutEvidence::PitchJump => "pitch_jump".to_string(),
        CutEvidence::LoudnessJump => "loudness_jump".to_string(),
        CutEvidence::SilenceGap => "silence_gap".to_string(),
        CutEvidence::MaxSpan => "max_span".to_string(),
    }
}

/// Load mono WAV and resample to 16 kHz when needed (benchmark fixtures may be 44.1 kHz).
#[cfg(any(test, feature = "bench"))]
pub fn load_wav_pcm_16k(path: &std::path::Path) -> anyhow::Result<Vec<f32>> {
    use crate::services::audio::resample_linear;
    use anyhow::Context;

    let mut reader = hound::WavReader::open(path).context("open WAV")?;
    let spec = reader.spec();
    if spec.channels != 1 {
        anyhow::bail!("expected mono WAV, got {} channels", spec.channels);
    }
    let pcm: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|x| x as f32 / 32768.0))
            .collect::<Result<_, _>>()
            .context("read int WAV samples")?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .context("read float WAV samples")?,
    };
    Ok(if spec.sample_rate == CHUNKING_SAMPLE_RATE {
        pcm
    } else {
        resample_linear(&pcm, spec.sample_rate, CHUNKING_SAMPLE_RATE)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_cuts_empty_when_disabled() {
        let pcm = vec![0.1f32; CHUNKING_SAMPLE_RATE as usize * 10];
        assert!(find_cuts(&pcm, CHUNKING_SAMPLE_RATE, false).is_empty());
    }

    #[test]
    fn find_cuts_skips_short_audio() {
        let pcm = vec![0.1f32; CHUNKING_SAMPLE_RATE as usize * 2];
        assert!(find_cuts(&pcm, CHUNKING_SAMPLE_RATE, true).is_empty());
    }

    #[test]
    fn split_pcm_single_chunk_without_cuts() {
        let pcm = vec![0.0f32; 1000];
        let chunks = split_pcm_owned(&pcm, &[], CHUNKING_SAMPLE_RATE);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].1.len(), 1000);
    }

    #[test]
    fn score_cuts_counts_hits() {
        let cuts = vec![
            SpeakerCut {
                time_s: 8.0,
                evidence: CutEvidence::PitchJump,
                strength: 5.0,
            },
            SpeakerCut {
                time_s: 22.0,
                evidence: CutEvidence::LoudnessJump,
                strength: 7.0,
            },
        ];
        let score = score_cuts(&cuts, &[7.67, 15.09, 22.5], 1.0);
        assert_eq!(score.caught, 2);
        assert_eq!(score.extra, 0);
    }

    /// Requires `SPEAKER_CHUNKING_FIXTURE` pointing at pitch_test benchmark WAV.
    #[test]
    #[ignore]
    fn benchmark_recall_on_fixture() {
        let path = std::env::var("SPEAKER_CHUNKING_FIXTURE")
            .expect("set SPEAKER_CHUNKING_FIXTURE to test_audio.wav");
        let pcm = load_wav_pcm_16k(std::path::Path::new(&path)).expect("read wav");
        let cuts = find_cuts(&pcm, CHUNKING_SAMPLE_RATE, true);
        let truth = [7.67, 15.09, 22.5, 32.42, 42.55];
        let score = score_cuts(&cuts, &truth, 1.0);
        assert!(
            score.caught >= 4,
            "recall {}/{} cuts={:?}",
            score.caught,
            score.total_truth,
            cuts
        );
    }
}
