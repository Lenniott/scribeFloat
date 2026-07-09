//! Live pitch/loudness analysis and voice-change-cut detection.
//!
//! Pure module: no I/O, no locks. `PitchAnalyzer` is fed 16 kHz mono samples by
//! the audio writer thread via `AudioService`'s PCM tap; `detect_cuts` runs once
//! at stop. A cut says "the voice changed here" — spans between cuts are NOT
//! speaker identities (identity is `VoiceprintService`'s job).
//!
//! Ported from the validated offline prototype (`pitch_speaker_cuts` CLI).
//! Benchmark on its test audio: caught 4/5 real changes, avg error 0.31 s,
//! 9 extra cuts, union mode — which is why union is the default and consensus
//! (caught 1/5) is an option only.

use crate::types::{AudioAnalysis, CutReason, SpeakerChangeCut};
use pitch_detection::detector::mcleod::McLeodDetector;
use pitch_detection::detector::PitchDetector;
use std::collections::BTreeSet;
use std::f32::consts::PI;

/// Must match `crate::services::audio::WHISPER_SAMPLE_RATE` — the tap delivers
/// post-resample samples, so frame times line up with mic.wav / Whisper times.
pub const ANALYSIS_SAMPLE_RATE: u32 = 16_000;
pub const WINDOW_SAMPLES: usize = 2048; // 128 ms — ~8 periods of a 65 Hz voice
pub const HOP_SAMPLES: usize = 1024; // 64 ms — finer than the 0.1 s eval grid

const FMIN_HZ: f32 = 65.0;
const FMAX_HZ: f32 = 400.0;
const RMS_VOICE_GATE: f32 = 1e-3; // ≈ −60 dBFS
const PEAK_VOICE_GATE: f32 = 0.01;
const MCLEOD_POWER_THRESHOLD: f64 = 0.002;
const MCLEOD_CLARITY_THRESHOLD: f64 = 0.6;

/// Tunables for [`detect_cuts`]. The default is the benchmarked configuration:
/// union mode (pitch OR loudness), silence cuts off, consensus off.
#[derive(Debug, Clone)]
pub struct CutConfig {
    /// Median window on each side of a candidate boundary.
    pub context_s: f32,
    /// How often candidate boundaries are evaluated.
    pub eval_step_s: f32,
    pub pitch_threshold_st: f32,
    pub loudness_threshold_db: f32,
    pub merge_cuts_within_s: f32,
    pub edge_guard_s: f32,
    /// Also cut at silence gaps (useful for aggressive transcript chunking,
    /// adds non-voice-change cuts). Off by default.
    pub include_silence: bool,
    /// Require >= 2 signals to agree. Precise but misses most real changes
    /// (benchmarked 1/5 vs union's 4/5). Off by default.
    pub consensus: bool,
    pub silence_top_db: f32,
    pub silence_min_gap_s: f32,
    /// Minimum voiced frames required on each side of a boundary.
    pub min_frames_per_side: usize,
}

impl Default for CutConfig {
    fn default() -> Self {
        Self {
            context_s: 1.0,
            eval_step_s: 0.1,
            pitch_threshold_st: 4.0,
            loudness_threshold_db: 6.0,
            merge_cuts_within_s: 0.75,
            edge_guard_s: 0.25,
            include_silence: false,
            consensus: false,
            silence_top_db: 25.0,
            silence_min_gap_s: 0.30,
            min_frames_per_side: 3,
        }
    }
}

/// Streaming windowed analyzer. Feed arbitrary-length 16 kHz mono chunks from
/// the writer-thread tap; call [`finish`](Self::finish) after the writer thread
/// has been joined.
pub struct PitchAnalyzer {
    buf: Vec<f32>,
    hann: Vec<f32>,
    scratch: Vec<f64>,
    f0_hz: Vec<Option<f32>>,
    rms: Vec<f32>,
}

impl PitchAnalyzer {
    pub fn new() -> Self {
        // The McLeodDetector is deliberately NOT a member: it holds Rc<RefCell<..>>
        // buffers internally, so owning one would make the analyzer !Send — and the
        // analyzer crosses threads (created by the controller, fed on the writer
        // thread, harvested by the controller). Constructing it per voiced window
        // costs a few short-lived Vec allocations at ~15.6 windows/s — negligible.
        // The Hann table and f64 scratch (the per-sample work) ARE reused.
        let hann = (0..WINDOW_SAMPLES)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / (WINDOW_SAMPLES - 1) as f32).cos())
            .collect();
        Self {
            buf: Vec::with_capacity(WINDOW_SAMPLES * 2),
            hann,
            scratch: vec![0.0; WINDOW_SAMPLES],
            f0_hz: Vec::new(),
            rms: Vec::new(),
        }
    }

    /// Accumulate samples; emits one frame per full hop-advanced window.
    pub fn feed(&mut self, samples: &[f32]) {
        self.buf.extend_from_slice(samples);
        while self.buf.len() >= WINDOW_SAMPLES {
            let window = &self.buf[..WINDOW_SAMPLES];
            let level = rms(window);
            let peak = peak_abs(window);

            let f0 = if level >= RMS_VOICE_GATE && peak >= PEAK_VOICE_GATE {
                for (dst, (&sample, &w)) in self
                    .scratch
                    .iter_mut()
                    .zip(window.iter().zip(self.hann.iter()))
                {
                    *dst = (sample * w) as f64;
                }
                McLeodDetector::new(WINDOW_SAMPLES, WINDOW_SAMPLES / 2)
                    .get_pitch(
                        &self.scratch,
                        ANALYSIS_SAMPLE_RATE as usize,
                        MCLEOD_POWER_THRESHOLD,
                        MCLEOD_CLARITY_THRESHOLD,
                    )
                    .map(|pitch| pitch.frequency as f32)
                    .filter(|hz| (FMIN_HZ..=FMAX_HZ).contains(hz))
            } else {
                None
            };

            self.f0_hz.push(f0);
            self.rms.push(level);
            self.buf.drain(..HOP_SAMPLES);
        }
    }

    /// Take the accumulated timeline. Trailing samples shorter than a window
    /// are dropped, matching the offline prototype.
    pub fn finish(&mut self) -> AudioAnalysis {
        AudioAnalysis {
            format_version: 1,
            sample_rate: ANALYSIS_SAMPLE_RATE,
            window_samples: WINDOW_SAMPLES as u32,
            hop_samples: HOP_SAMPLES as u32,
            f0_hz: std::mem::take(&mut self.f0_hz),
            rms: std::mem::take(&mut self.rms),
        }
    }
}

impl Default for PitchAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Root-mean-square level. Canonical RMS for the crate — other modules
/// (hallucination gating, level metering) should delegate here.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / samples.len() as f32).sqrt()
}

pub fn peak_abs(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0_f32, |acc, sample| acc.max(sample.abs()))
}

/// Semitones relative to 100 Hz, so equal pitch ratios become equal distances.
pub fn hz_to_semitones(hz: f32) -> f32 {
    12.0 * (hz / 100.0).log2()
}

/// One frame of the timeline, reconstituted from `AudioAnalysis`'s parallel
/// arrays for the cut detectors.
struct Frame {
    time_s: f32,
    f0_hz: Option<f32>,
    rms: f32,
}

fn frames_of(analysis: &AudioAnalysis) -> Vec<Frame> {
    let hop = analysis.hop_samples as f32;
    let half_window = analysis.window_samples as f32 / 2.0;
    let sr = analysis.sample_rate as f32;
    analysis
        .f0_hz
        .iter()
        .zip(analysis.rms.iter())
        .enumerate()
        .map(|(i, (&f0_hz, &rms))| Frame {
            time_s: (i as f32 * hop + half_window) / sr,
            f0_hz,
            rms,
        })
        .collect()
}

/// Duration covered by the analyzed windows (trailing partial window excluded).
fn duration_s(analysis: &AudioAnalysis) -> f32 {
    if analysis.f0_hz.is_empty() {
        return 0.0;
    }
    let samples = (analysis.f0_hz.len() - 1) as u64 * analysis.hop_samples as u64
        + analysis.window_samples as u64;
    samples as f32 / analysis.sample_rate as f32
}

/// Detect voice-change cuts over a completed timeline.
pub fn detect_cuts(analysis: &AudioAnalysis, cfg: &CutConfig) -> Vec<SpeakerChangeCut> {
    let frames = frames_of(analysis);
    let duration = duration_s(analysis);
    let frame_dt = analysis.hop_samples as f32 / analysis.sample_rate as f32;

    let mut candidates = Vec::new();
    candidates.extend(pitch_cuts(&frames, frame_dt, cfg));
    candidates.extend(loudness_cuts(&frames, frame_dt, cfg));
    if cfg.include_silence {
        candidates.extend(silence_cuts(&frames, frame_dt, cfg));
    }

    merge_cuts(candidates, cfg.merge_cuts_within_s)
        .into_iter()
        .filter(|cut| cut.time_s >= cfg.edge_guard_s && cut.time_s <= duration - cfg.edge_guard_s)
        .filter(|cut| !cfg.consensus || cut.reasons.len() >= 2)
        .collect()
}

fn eval_step_frames(frame_dt: f32, cfg: &CutConfig) -> usize {
    (cfg.eval_step_s / frame_dt).round().max(1.0) as usize
}

fn context_frames(frame_dt: f32, cfg: &CutConfig) -> usize {
    (cfg.context_s / frame_dt).round().max(1.0) as usize
}

fn pitch_cuts(frames: &[Frame], frame_dt: f32, cfg: &CutConfig) -> Vec<SpeakerChangeCut> {
    let voiced: Vec<(usize, f32)> = frames
        .iter()
        .enumerate()
        .filter_map(|(i, frame)| frame.f0_hz.map(|hz| (i, hz_to_semitones(hz))))
        .collect();
    let step = eval_step_frames(frame_dt, cfg);
    let win = context_frames(frame_dt, cfg);
    let mut cuts = Vec::new();

    for i in (0..frames.len()).step_by(step) {
        let k = voiced.partition_point(|(idx, _)| *idx < i);
        let before: Vec<f32> = voiced[k.saturating_sub(win)..k]
            .iter()
            .map(|(_, st)| *st)
            .collect();
        let after: Vec<f32> = voiced[k..(k + win).min(voiced.len())]
            .iter()
            .map(|(_, st)| *st)
            .collect();
        if before.len() < cfg.min_frames_per_side || after.len() < cfg.min_frames_per_side {
            continue;
        }
        let jump = (median(before) - median(after)).abs();
        if jump > cfg.pitch_threshold_st {
            cuts.push(single_reason_cut(
                frames[i].time_s,
                jump / cfg.pitch_threshold_st,
                CutReason::Pitch,
            ));
        }
    }

    merge_cuts(cuts, cfg.eval_step_s * 2.0)
}

fn loudness_cuts(frames: &[Frame], frame_dt: f32, cfg: &CutConfig) -> Vec<SpeakerChangeCut> {
    let voiced: Vec<usize> = frames
        .iter()
        .enumerate()
        .filter_map(|(i, frame)| frame.f0_hz.map(|_| i))
        .collect();
    let step = eval_step_frames(frame_dt, cfg);
    let win = context_frames(frame_dt, cfg);
    let mut cuts = Vec::new();

    for i in (0..frames.len()).step_by(step) {
        let k = voiced.partition_point(|idx| *idx < i);
        let before_idx = &voiced[k.saturating_sub(win)..k];
        let after_idx = &voiced[k..(k + win).min(voiced.len())];
        if before_idx.len() < cfg.min_frames_per_side || after_idx.len() < cfg.min_frames_per_side {
            continue;
        }

        let before: Vec<f32> = before_idx
            .iter()
            .map(|idx| frames[*idx].rms.max(1e-9))
            .collect();
        let after: Vec<f32> = after_idx
            .iter()
            .map(|idx| frames[*idx].rms.max(1e-9))
            .collect();
        let jump = (20.0 * (median(after) / median(before)).log10()).abs();
        if jump > cfg.loudness_threshold_db {
            cuts.push(single_reason_cut(
                frames[i].time_s,
                jump / cfg.loudness_threshold_db,
                CutReason::Loudness,
            ));
        }
    }

    merge_cuts(cuts, cfg.eval_step_s * 2.0)
}

fn silence_cuts(frames: &[Frame], frame_dt: f32, cfg: &CutConfig) -> Vec<SpeakerChangeCut> {
    let peak_rms = frames.iter().map(|f| f.rms).fold(0.0_f32, f32::max);
    if peak_rms <= 0.0 {
        return Vec::new();
    }
    let quiet_threshold = peak_rms * 10.0_f32.powf(-cfg.silence_top_db / 20.0);

    let mut cuts = Vec::new();
    let mut quiet_start = None;
    for (i, frame) in frames.iter().enumerate() {
        if frame.rms <= quiet_threshold {
            quiet_start.get_or_insert(i);
        } else if let Some(start) = quiet_start.take() {
            let gap = (i - start) as f32 * frame_dt;
            if gap >= cfg.silence_min_gap_s {
                let mid = (frames[start].time_s + frames[i.saturating_sub(1)].time_s) / 2.0;
                cuts.push(single_reason_cut(
                    mid,
                    gap / cfg.silence_min_gap_s,
                    CutReason::Silence,
                ));
            }
        }
    }
    cuts
}

fn single_reason_cut(time_s: f32, score: f32, reason: CutReason) -> SpeakerChangeCut {
    let mut reasons = BTreeSet::new();
    reasons.insert(reason);
    SpeakerChangeCut {
        time_s,
        end_s: time_s,
        score,
        reasons,
    }
}

/// Collapse candidates within `within_s` of each other into one cut, keeping
/// the highest-scoring member's time and the union of reasons.
fn merge_cuts(mut cuts: Vec<SpeakerChangeCut>, within_s: f32) -> Vec<SpeakerChangeCut> {
    cuts.sort_by(|a, b| a.time_s.total_cmp(&b.time_s));
    let mut merged: Vec<SpeakerChangeCut> = Vec::new();

    for cut in cuts {
        if let Some(last) = merged.last_mut() {
            if cut.time_s - last.end_s <= within_s {
                last.end_s = cut.time_s;
                last.reasons.extend(cut.reasons.iter().copied());
                if cut.score > last.score {
                    last.time_s = cut.time_s;
                    last.score = cut.score;
                }
                continue;
            }
        }
        merged.push(cut);
    }

    merged
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq_hz: f32, seconds: f32, amplitude: f32) -> Vec<f32> {
        let n = (seconds * ANALYSIS_SAMPLE_RATE as f32) as usize;
        (0..n)
            .map(|i| {
                (2.0 * PI * freq_hz * i as f32 / ANALYSIS_SAMPLE_RATE as f32).sin() * amplitude
            })
            .collect()
    }

    /// Build a timeline directly (frame-level tests need no audio).
    fn analysis_from(f0_hz: Vec<Option<f32>>, rms: Vec<f32>) -> AudioAnalysis {
        AudioAnalysis {
            format_version: 1,
            sample_rate: ANALYSIS_SAMPLE_RATE,
            window_samples: WINDOW_SAMPLES as u32,
            hop_samples: HOP_SAMPLES as u32,
            f0_hz,
            rms,
        }
    }

    #[test]
    fn semitone_reference_points() {
        assert!((hz_to_semitones(100.0) - 0.0).abs() < 0.001);
        assert!((hz_to_semitones(200.0) - 12.0).abs() < 0.001);
        assert!((hz_to_semitones(50.0) + 12.0).abs() < 0.001);
    }

    #[test]
    fn rms_matches_known_signal() {
        // Full-scale sine has RMS 1/sqrt(2).
        let wave = sine(100.0, 1.0, 1.0);
        assert!((rms(&wave) - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01);
        assert_eq!(rms(&[]), 0.0);
    }

    #[test]
    fn mcleod_detects_16k_sine() {
        let mut analyzer = PitchAnalyzer::new();
        analyzer.feed(&sine(220.0, WINDOW_SAMPLES as f32 / 16_000.0, 0.2));
        let analysis = analyzer.finish();
        assert_eq!(analysis.f0_hz.len(), 1);
        let hz = analysis.f0_hz[0].expect("voiced");
        assert!((hz - 220.0).abs() < 10.0, "{hz}");
    }

    #[test]
    fn analyzer_emits_expected_frame_count() {
        // 2 s fed in cpal-ish 480-sample chunks: (32000 - 2048) / 1024 + 1 = 30.
        let wave = sine(150.0, 2.0, 0.2);
        let mut analyzer = PitchAnalyzer::new();
        for chunk in wave.chunks(480) {
            analyzer.feed(chunk);
        }
        let analysis = analyzer.finish();
        assert_eq!(analysis.f0_hz.len(), 30);
        assert_eq!(analysis.rms.len(), 30);
    }

    #[test]
    fn analyzer_marks_silence_unvoiced() {
        let mut analyzer = PitchAnalyzer::new();
        analyzer.feed(&vec![0.0; WINDOW_SAMPLES * 4]);
        let analysis = analyzer.finish();
        assert!(!analysis.f0_hz.is_empty());
        assert!(analysis.f0_hz.iter().all(|f0| f0.is_none()));
        assert!(analysis.rms.iter().all(|&r| r < 1e-6));
    }

    #[test]
    fn analyzer_gates_out_of_band_pitch() {
        for freq in [50.0, 1000.0] {
            let mut analyzer = PitchAnalyzer::new();
            analyzer.feed(&sine(freq, 1.0, 0.2));
            let analysis = analyzer.finish();
            assert!(
                analysis.f0_hz.iter().all(|f0| f0.is_none()),
                "{freq} Hz should be outside the 65-400 Hz voice band"
            );
        }
    }

    #[test]
    fn detects_pitch_jump_from_synthetic_frames() {
        let f0 = (0..80)
            .map(|i| Some(if i < 40 { 110.0 } else { 220.0 }))
            .collect();
        let cuts = detect_cuts(&analysis_from(f0, vec![0.05; 80]), &CutConfig::default());
        assert_eq!(cuts.len(), 1);
        assert!(cuts[0].reasons.contains(&CutReason::Pitch));
        assert!((2.0..=2.8).contains(&cuts[0].time_s), "{}", cuts[0].time_s);
        assert!(cuts[0].score >= 1.0);
    }

    #[test]
    fn detects_loudness_jump_from_synthetic_frames() {
        let rms = (0..80).map(|i| if i < 40 { 0.01 } else { 0.08 }).collect();
        let cuts = detect_cuts(
            &analysis_from(vec![Some(120.0); 80], rms),
            &CutConfig::default(),
        );
        assert_eq!(cuts.len(), 1);
        assert!(cuts[0].reasons.contains(&CutReason::Loudness));
    }

    #[test]
    fn stable_audio_has_no_cuts() {
        let cuts = detect_cuts(
            &analysis_from(vec![Some(120.0); 80], vec![0.05; 80]),
            &CutConfig::default(),
        );
        assert!(cuts.is_empty());
    }

    #[test]
    fn consensus_mode_requires_two_signals() {
        let cfg = CutConfig {
            consensus: true,
            ..CutConfig::default()
        };
        let f0: Vec<Option<f32>> = (0..80)
            .map(|i| Some(if i < 40 { 110.0 } else { 220.0 }))
            .collect();

        // Pitch jump alone does not survive consensus.
        assert!(detect_cuts(&analysis_from(f0.clone(), vec![0.05; 80]), &cfg).is_empty());

        // Pitch + loudness jump together do.
        let rms = (0..80).map(|i| if i < 40 { 0.01 } else { 0.08 }).collect();
        let cuts = detect_cuts(&analysis_from(f0, rms), &cfg);
        assert_eq!(cuts.len(), 1);
        assert!(cuts[0].reasons.contains(&CutReason::Pitch));
        assert!(cuts[0].reasons.contains(&CutReason::Loudness));
    }

    #[test]
    fn cuts_near_edges_are_dropped() {
        // Jump at frame 3 (~0.26 s) is inside the default context window, and
        // even if detected it must be edge-guarded... use a short timeline where
        // the only jump sits within edge_guard_s of the start.
        let f0: Vec<Option<f32>> = (0..8)
            .map(|i| Some(if i < 2 { 110.0 } else { 220.0 }))
            .collect();
        let cfg = CutConfig {
            context_s: 0.128, // 2 frames
            min_frames_per_side: 2,
            ..CutConfig::default()
        };
        let cuts = detect_cuts(&analysis_from(f0, vec![0.05; 8]), &cfg);
        assert!(
            cuts.iter().all(|c| c.time_s >= cfg.edge_guard_s),
            "cuts within the edge guard must be dropped: {cuts:?}"
        );
    }

    #[test]
    fn silence_cuts_only_when_enabled() {
        // 0.5 s of speech, ~0.6 s gap, 0.5 s of speech (frame-level).
        let mut f0 = Vec::new();
        let mut rms = Vec::new();
        for i in 0..40 {
            let quiet = (15..25).contains(&i);
            f0.push(if quiet { None } else { Some(120.0) });
            rms.push(if quiet { 0.0001 } else { 0.05 });
        }
        let analysis = analysis_from(f0, rms);
        let default_cuts = detect_cuts(&analysis, &CutConfig::default());
        assert!(
            default_cuts
                .iter()
                .all(|c| !c.reasons.contains(&CutReason::Silence)),
            "silence cuts must be off by default"
        );
        let cfg = CutConfig {
            include_silence: true,
            ..CutConfig::default()
        };
        let cuts = detect_cuts(&analysis, &cfg);
        assert!(
            cuts.iter().any(|c| c.reasons.contains(&CutReason::Silence)),
            "expected a silence cut: {cuts:?}"
        );
    }

    #[test]
    fn end_to_end_pitch_jump_in_real_audio() {
        // 2 s at 110 Hz then 2 s at 220 Hz, same level -> exactly one Pitch cut
        // at 2.0 +/- 0.2 s.
        let mut wave = sine(110.0, 2.0, 0.2);
        wave.extend(sine(220.0, 2.0, 0.2));
        let mut analyzer = PitchAnalyzer::new();
        for chunk in wave.chunks(480) {
            analyzer.feed(chunk);
        }
        let analysis = analyzer.finish();
        let cuts = detect_cuts(&analysis, &CutConfig::default());
        assert_eq!(cuts.len(), 1, "{cuts:?}");
        assert!(cuts[0].reasons.contains(&CutReason::Pitch));
        assert!((cuts[0].time_s - 2.0).abs() <= 0.2, "{}", cuts[0].time_s);
    }
}
