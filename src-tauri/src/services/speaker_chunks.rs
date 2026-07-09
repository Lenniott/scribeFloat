use crate::services::analysis::rms;
use crate::services::voiceprint::VoiceprintService;
use crate::types::{
    LabelCorrection, Segment, SessionSpeaker, SpeakerBlock, SpeakerChangeCut, SpeakerChunk,
    VoiceprintProfile,
};

const MIN_EMBED_MS: u64 = 2_000;
const MIN_TURN_CUT_MS: u64 = 1_500;
const VAD_FRAME_SAMPLES: usize = 1024;
const SPEECH_RMS_GATE: f32 = 1e-3;
const CLIP_GATE: f32 = 0.999;
const LOCAL_CLUSTER_THRESHOLD: f32 = 0.60;
const CLEAN_CHUNK_MIN_PURITY: f32 = 0.60;
const LOCAL_SPEAKER_PREFIX: &str = "Speaker ";
const OTHER_LABEL: &str = "Other";
/// Chunks with a smaller best-vs-second-best margin are genuinely ambiguous
/// and are never auto-relabeled by the correction cascade.
const CASCADE_MIN_MARGIN: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkSpan {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl ChunkSpan {
    pub fn id(&self, index: usize) -> String {
        format!("chunk-{:04}", index + 1)
    }

    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

#[derive(Debug, Clone, Copy)]
struct ChunkQuality {
    audio_duration_s: f32,
    vad_purity: f32,
    rms_energy: f32,
    clipping: bool,
}

#[derive(Debug, Clone)]
struct Cluster {
    id: String,
    centroid: Vec<f32>,
    members: Vec<usize>,
    label: String,
    matched_profile: Option<String>,
    profile_score: Option<f32>,
}

/// Build voice-turn spans from detected speaker-change cuts. CAMPPlus needs
/// about 2 seconds of audio, but the cut detector can land a real 2-second turn
/// slightly early. Keep those near-boundary cuts by snapping them to 2 seconds.
pub fn build_chunk_spans(cuts: &[SpeakerChangeCut], total_ms: u64) -> Vec<ChunkSpan> {
    if total_ms == 0 {
        return Vec::new();
    }

    let mut cut_ms: Vec<u64> = cuts
        .iter()
        .map(|cut| (cut.time_s.max(0.0) * 1000.0).round() as u64)
        .filter(|ms| *ms > 0 && *ms < total_ms)
        .collect();
    cut_ms.sort_unstable();
    cut_ms.dedup();

    let mut spans = Vec::new();
    let mut start = 0;
    for cut in cut_ms {
        let span_ms = cut.saturating_sub(start);
        let boundary = if span_ms >= MIN_EMBED_MS {
            Some(cut)
        } else if span_ms >= MIN_TURN_CUT_MS {
            Some((start + MIN_EMBED_MS).min(total_ms))
        } else {
            None
        };

        if let Some(boundary) =
            boundary.filter(|boundary| *boundary > start && *boundary < total_ms)
        {
            spans.push(ChunkSpan {
                start_ms: start,
                end_ms: boundary,
            });
            start = boundary;
        }
    }
    spans.push(ChunkSpan {
        start_ms: start,
        end_ms: total_ms,
    });

    if spans.len() > 1
        && spans
            .last()
            .is_some_and(|span| span.duration_ms() < MIN_TURN_CUT_MS)
    {
        let last = spans.pop().expect("last span exists");
        if let Some(prev) = spans.last_mut() {
            prev.end_ms = last.end_ms;
        }
    }

    spans
}

pub fn analyze_chunks(
    pcm: &[f32],
    sample_rate: u32,
    cuts: &[SpeakerChangeCut],
    voiceprint: &VoiceprintService,
    profile_threshold: f32,
) -> Vec<SpeakerChunk> {
    let total_ms = samples_to_ms(pcm.len(), sample_rate);
    let spans = build_chunk_spans(cuts, total_ms);
    let profiles = match voiceprint.load_profiles() {
        Ok(profiles) => profiles,
        Err(err) => {
            tracing::warn!(error = %err, "speaker chunk profile load failed");
            Vec::new()
        }
    };

    let mut chunks: Vec<SpeakerChunk> = spans
        .iter()
        .enumerate()
        .map(|(index, span)| {
            let slice = pcm_slice(pcm, sample_rate, *span);
            let quality = chunk_quality(slice, sample_rate);
            let embedding = if span.duration_ms() >= MIN_EMBED_MS {
                match voiceprint.embed(slice, sample_rate) {
                    Ok(embedding) => Some(embedding),
                    Err(err) => {
                        tracing::debug!(
                            chunk_id = %span.id(index),
                            error = %err,
                            "speaker chunk embedding skipped"
                        );
                        None
                    }
                }
            } else {
                None
            };

            SpeakerChunk {
                id: span.id(index),
                start_ms: span.start_ms,
                end_ms: span.end_ms,
                label: OTHER_LABEL.to_string(),
                cluster_id: None,
                matched_profile: None,
                embedding,
                encrypted_embedding: None,
                audio_duration_s: quality.audio_duration_s,
                vad_purity: quality.vad_purity,
                rms_energy: quality.rms_energy,
                clipping: quality.clipping,
                profile_score: None,
                session_score: None,
                margin: None,
                corrections: Vec::new(),
            }
        })
        .collect();

    let clusters = cluster_chunks(&chunks, &profiles, profile_threshold);
    for cluster in clusters {
        for member in cluster.members {
            if let Some(chunk) = chunks.get_mut(member) {
                chunk.cluster_id = Some(cluster.id.clone());
                chunk.label = cluster.label.clone();
                chunk.matched_profile = cluster.matched_profile.clone();
                chunk.profile_score = cluster.profile_score;
            }
        }
    }

    chunks
}

pub fn build_blocks_from_chunks(
    segments: &[Segment],
    chunks: &[SpeakerChunk],
) -> Vec<SpeakerBlock> {
    if chunks.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        let Some(chunk) = chunk_for_segment(segment, chunks) else {
            continue;
        };
        blocks.push(SpeakerBlock {
            label: chunk.label.clone(),
            start_ms: Some(segment.start_ms.max(0) as u64),
            end_ms: Some(segment.end_ms.max(0) as u64),
            text: text.to_string(),
            chunk_id: Some(chunk.id.clone()),
        });
    }

    merge_blocks_same_label_and_chunk(blocks)
}

pub fn build_session_speakers(chunks: &[SpeakerChunk]) -> Vec<SessionSpeaker> {
    let mut groups: Vec<(String, Vec<&SpeakerChunk>)> = Vec::new();
    for chunk in chunks
        .iter()
        .filter(|chunk| clean_for_session_speaker(chunk))
    {
        let id = chunk.cluster_id.clone().unwrap_or_else(|| chunk.id.clone());
        if let Some((_, members)) = groups.iter_mut().find(|(group_id, _)| *group_id == id) {
            members.push(chunk);
        } else {
            groups.push((id, vec![chunk]));
        }
    }

    groups
        .into_iter()
        .filter_map(|(session_speaker_id, members)| {
            session_speaker_from_members(session_speaker_id, &members)
        })
        .collect()
}

fn session_speaker_from_members(
    session_speaker_id: String,
    members: &[&SpeakerChunk],
) -> Option<SessionSpeaker> {
    let centroid_embedding = weighted_centroid(members)?;
    let start_ms = members
        .iter()
        .map(|chunk| chunk.start_ms)
        .min()
        .unwrap_or(0);
    let end_ms = members
        .iter()
        .map(|chunk| chunk.end_ms)
        .max()
        .unwrap_or(start_ms);
    let duration_ms = members.iter().map(|chunk| chunk_duration_ms(chunk)).sum();
    let radius = speaker_radius(&centroid_embedding, members);
    let quality_score = speaker_quality_score(members, radius);
    let label = members
        .iter()
        .find(|chunk| chunk.label != OTHER_LABEL)
        .or_else(|| members.first())
        .map(|chunk| chunk.label.clone())
        .unwrap_or_else(|| OTHER_LABEL.to_string());

    Some(SessionSpeaker {
        session_speaker_id,
        label,
        centroid_embedding,
        encrypted_centroid_embedding: None,
        clean_chunk_ids: members.iter().map(|chunk| chunk.id.clone()).collect(),
        start_ms,
        end_ms,
        duration_ms,
        radius,
        quality_score,
        user_confirmed: false,
    })
}

/// Recompute one session speaker from the current chunk membership, keeping
/// its existing label and confirmation. Removes the speaker when the group no
/// longer has any clean embedded member.
fn rebuild_session_speaker(
    chunks: &[SpeakerChunk],
    speakers: &mut Vec<SessionSpeaker>,
    group_id: &str,
) {
    let members: Vec<&SpeakerChunk> = chunks
        .iter()
        .filter(|chunk| {
            clean_for_session_speaker(chunk) && chunk.cluster_id.as_deref() == Some(group_id)
        })
        .collect();
    let existing = speakers
        .iter()
        .position(|speaker| speaker.session_speaker_id == group_id);

    match session_speaker_from_members(group_id.to_string(), &members) {
        Some(mut rebuilt) => {
            if let Some(idx) = existing {
                rebuilt.label = speakers[idx].label.clone();
                rebuilt.user_confirmed = speakers[idx].user_confirmed;
                speakers[idx] = rebuilt;
            } else {
                speakers.push(rebuilt);
            }
        }
        None => {
            if let Some(idx) = existing {
                speakers.remove(idx);
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CorrectionOutcome {
    pub corrected_chunk_id: Option<String>,
    pub auto_corrected_chunk_ids: Vec<String>,
}

/// Apply a user label correction to one chunk, then cascade: recompute the two
/// affected centroids, re-score every chunk, and auto-relabel chunks whose
/// winning centroid changed — unless their margin marks them genuinely
/// ambiguous or they carry an explicit user correction.
pub fn correct_chunk_label(
    chunks: &mut [SpeakerChunk],
    speakers: &mut Vec<SessionSpeaker>,
    chunk_id: &str,
    new_label: &str,
    now_ms: u64,
) -> Result<CorrectionOutcome, String> {
    let idx = chunks
        .iter()
        .position(|chunk| chunk.id == chunk_id)
        .ok_or_else(|| format!("unknown chunk `{chunk_id}`"))?;
    let old_label = chunks[idx].label.clone();
    if old_label == new_label {
        return Ok(CorrectionOutcome::default());
    }

    let old_cluster = chunks[idx].cluster_id.clone();
    let target_cluster = speakers
        .iter()
        .find(|speaker| speaker.label == new_label)
        .map(|speaker| speaker.session_speaker_id.clone())
        .unwrap_or_else(|| next_cluster_id(chunks, speakers));

    chunks[idx].label = new_label.to_string();
    chunks[idx].cluster_id = Some(target_cluster.clone());
    chunks[idx].corrections.push(LabelCorrection {
        from_label: old_label,
        to_label: new_label.to_string(),
        corrected_at_ms: now_ms,
        auto: false,
    });

    if let Some(old_cluster) = old_cluster.as_deref().filter(|id| *id != target_cluster) {
        rebuild_session_speaker(chunks, speakers, old_cluster);
    }
    rebuild_session_speaker(chunks, speakers, &target_cluster);
    if let Some(target) = speakers
        .iter_mut()
        .find(|speaker| speaker.session_speaker_id == target_cluster)
    {
        target.label = new_label.to_string();
        target.user_confirmed = true;
    }
    score_chunks(chunks, speakers);

    let mut auto_corrected_chunk_ids = Vec::new();
    let mut touched_groups = Vec::new();
    for (move_idx, winner_idx) in cascade_moves(chunks, speakers) {
        let winner_id = speakers[winner_idx].session_speaker_id.clone();
        let winner_label = speakers[winner_idx].label.clone();
        let chunk = &mut chunks[move_idx];
        if let Some(previous) = chunk.cluster_id.clone() {
            touched_groups.push(previous);
        }
        touched_groups.push(winner_id.clone());
        chunk.corrections.push(LabelCorrection {
            from_label: chunk.label.clone(),
            to_label: winner_label.clone(),
            corrected_at_ms: now_ms,
            auto: true,
        });
        chunk.label = winner_label;
        chunk.cluster_id = Some(winner_id);
        auto_corrected_chunk_ids.push(chunk.id.clone());
    }

    if !auto_corrected_chunk_ids.is_empty() {
        touched_groups.sort();
        touched_groups.dedup();
        for group in &touched_groups {
            rebuild_session_speaker(chunks, speakers, group);
        }
        score_chunks(chunks, speakers);
    }

    Ok(CorrectionOutcome {
        corrected_chunk_id: Some(chunk_id.to_string()),
        auto_corrected_chunk_ids,
    })
}

/// Chunks whose winning centroid differs from their current cluster and whose
/// margin clears the ambiguity gate. Chunks the user explicitly corrected are
/// never moved.
fn cascade_moves(chunks: &[SpeakerChunk], speakers: &[SessionSpeaker]) -> Vec<(usize, usize)> {
    chunks
        .iter()
        .enumerate()
        .filter_map(|(idx, chunk)| {
            if chunk.corrections.iter().any(|correction| !correction.auto) {
                return None;
            }
            let embedding = chunk.embedding.as_deref()?;
            if chunk.margin? < CASCADE_MIN_MARGIN {
                return None;
            }
            let (winner_idx, _) = speakers
                .iter()
                .enumerate()
                .map(|(speaker_idx, speaker)| {
                    (speaker_idx, cosine(embedding, &speaker.centroid_embedding))
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))?;
            let winner_id = &speakers[winner_idx].session_speaker_id;
            (chunk.cluster_id.as_ref() != Some(winner_id)).then_some((idx, winner_idx))
        })
        .collect()
}

fn next_cluster_id(chunks: &[SpeakerChunk], speakers: &[SessionSpeaker]) -> String {
    let max_suffix = chunks
        .iter()
        .filter_map(|chunk| chunk.cluster_id.as_deref())
        .chain(
            speakers
                .iter()
                .map(|speaker| speaker.session_speaker_id.as_str()),
        )
        .filter_map(|id| id.strip_prefix("speaker-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("speaker-{}", max_suffix + 1)
}

/// Back-fill `session_score` and `margin` onto each embedded chunk from the
/// session-speaker centroids. Pure cosine math — no re-embedding.
pub fn score_chunks(chunks: &mut [SpeakerChunk], speakers: &[SessionSpeaker]) {
    for chunk in chunks {
        let Some(embedding) = chunk.embedding.as_deref() else {
            continue;
        };

        chunk.session_score = speakers
            .iter()
            .find(|speaker| Some(&speaker.session_speaker_id) == chunk.cluster_id.as_ref())
            .map(|speaker| cosine(embedding, &speaker.centroid_embedding));

        let mut scores: Vec<f32> = speakers
            .iter()
            .map(|speaker| cosine(embedding, &speaker.centroid_embedding))
            .collect();
        scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        chunk.margin = match scores.as_slice() {
            [best, second, ..] => Some(best - second),
            _ => None,
        };
    }
}

pub fn pcm_slice(pcm: &[f32], sample_rate: u32, span: ChunkSpan) -> &[f32] {
    let start = ms_to_samples(span.start_ms, sample_rate).min(pcm.len());
    let end = ms_to_samples(span.end_ms, sample_rate).min(pcm.len());
    &pcm[start..end.max(start)]
}

fn clean_for_session_speaker(chunk: &SpeakerChunk) -> bool {
    chunk.embedding.is_some()
        && chunk.audio_duration_s >= MIN_EMBED_MS as f32 / 1000.0
        && chunk.vad_purity >= CLEAN_CHUNK_MIN_PURITY
        && !chunk.clipping
}

fn weighted_centroid(chunks: &[&SpeakerChunk]) -> Option<Vec<f32>> {
    let first = chunks.first()?.embedding.as_ref()?;
    let len = first.len();
    let mut centroid = vec![0.0; len];
    let mut total_weight = 0.0_f32;
    for chunk in chunks {
        let embedding = chunk.embedding.as_deref()?;
        if embedding.len() != len {
            continue;
        }
        let weight = chunk_duration_ms(chunk).max(1) as f32;
        total_weight += weight;
        for (dst, value) in centroid.iter_mut().zip(embedding.iter()) {
            *dst += *value * weight;
        }
    }
    if total_weight <= 0.0 {
        return None;
    }
    for value in &mut centroid {
        *value /= total_weight;
    }
    Some(l2_normalize(centroid))
}

fn speaker_radius(centroid: &[f32], chunks: &[&SpeakerChunk]) -> f32 {
    let mut distances = Vec::new();
    for chunk in chunks {
        if let Some(embedding) = chunk.embedding.as_deref() {
            distances.push(1.0 - cosine(centroid, embedding));
        }
    }
    if distances.is_empty() {
        return 0.0;
    }
    distances.iter().copied().fold(0.0_f32, f32::max).max(0.0)
}

fn speaker_quality_score(chunks: &[&SpeakerChunk], radius: f32) -> f32 {
    let total_duration = chunks
        .iter()
        .map(|chunk| chunk_duration_ms(chunk).max(1) as f32)
        .sum::<f32>();
    if total_duration <= 0.0 {
        return 0.0;
    }
    let purity = chunks
        .iter()
        .map(|chunk| chunk.vad_purity * chunk_duration_ms(chunk).max(1) as f32)
        .sum::<f32>()
        / total_duration;
    (purity * (1.0 - radius).clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

fn chunk_duration_ms(chunk: &SpeakerChunk) -> u64 {
    chunk.end_ms.saturating_sub(chunk.start_ms)
}

fn cluster_chunks(
    chunks: &[SpeakerChunk],
    profiles: &[VoiceprintProfile],
    profile_threshold: f32,
) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = Vec::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let Some(embedding) = chunk.embedding.as_deref() else {
            continue;
        };

        let best = clusters
            .iter()
            .enumerate()
            .map(|(cluster_index, cluster)| (cluster_index, cosine(embedding, &cluster.centroid)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((cluster_index, _)) =
            best.filter(|(_, score)| *score >= LOCAL_CLUSTER_THRESHOLD)
        {
            let cluster = &mut clusters[cluster_index];
            cluster.members.push(index);
            cluster.centroid = centroid_for_members(chunks, &cluster.members);
        } else {
            let speaker_index = clusters.len();
            clusters.push(Cluster {
                id: format!("speaker-{}", speaker_index + 1),
                centroid: embedding.to_vec(),
                members: vec![index],
                label: local_speaker_label(speaker_index),
                matched_profile: None,
                profile_score: None,
            });
        }
    }

    for (index, cluster) in clusters.iter_mut().enumerate() {
        let (matched_profile, profile_score) =
            best_profile_match(&cluster.centroid, profiles, profile_threshold);
        cluster.matched_profile = matched_profile.clone();
        cluster.profile_score = profile_score;
        cluster.label = matched_profile.unwrap_or_else(|| local_speaker_label(index));
    }

    clusters
}

fn centroid_for_members(chunks: &[SpeakerChunk], members: &[usize]) -> Vec<f32> {
    let embeddings: Vec<&[f32]> = members
        .iter()
        .filter_map(|index| chunks.get(*index)?.embedding.as_deref())
        .collect();
    if embeddings.is_empty() {
        return Vec::new();
    }
    let len = embeddings[0].len();
    let mut centroid = vec![0.0; len];
    for embedding in &embeddings {
        if embedding.len() != len {
            continue;
        }
        for (dst, value) in centroid.iter_mut().zip(embedding.iter()) {
            *dst += *value;
        }
    }
    for value in &mut centroid {
        *value /= embeddings.len() as f32;
    }
    l2_normalize(centroid)
}

fn best_profile_match(
    embedding: &[f32],
    profiles: &[VoiceprintProfile],
    threshold: f32,
) -> (Option<String>, Option<f32>) {
    profiles
        .iter()
        .filter(|profile| profile.embedding.len() == embedding.len())
        .map(|profile| (profile.name.clone(), cosine(embedding, &profile.embedding)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(name, score)| {
            if score >= threshold {
                (Some(name), Some(score))
            } else {
                (None, Some(score))
            }
        })
        .unwrap_or((None, None))
}

fn chunk_for_segment<'a>(
    segment: &Segment,
    chunks: &'a [SpeakerChunk],
) -> Option<&'a SpeakerChunk> {
    let start = segment.start_ms.max(0) as u64;
    let end = segment.end_ms.max(segment.start_ms).max(0) as u64;
    let best_overlap = chunks
        .iter()
        .map(|chunk| (chunk, overlap_ms(start, end, chunk.start_ms, chunk.end_ms)))
        .max_by_key(|(_, overlap)| *overlap);

    if let Some((chunk, _)) = best_overlap.filter(|(_, overlap)| *overlap > 0) {
        return Some(chunk);
    }

    let mid_ms = start + end.saturating_sub(start) / 2;
    chunks
        .iter()
        .find(|chunk| mid_ms >= chunk.start_ms && mid_ms < chunk.end_ms)
        .or_else(|| chunks.last().filter(|chunk| mid_ms == chunk.end_ms))
}

fn overlap_ms(a_start: u64, a_end: u64, b_start: u64, b_end: u64) -> u64 {
    a_end.min(b_end).saturating_sub(a_start.max(b_start))
}

fn merge_blocks_same_label_and_chunk(blocks: Vec<SpeakerBlock>) -> Vec<SpeakerBlock> {
    let mut merged: Vec<SpeakerBlock> = Vec::new();
    for block in blocks {
        if let Some(last) = merged
            .last_mut()
            .filter(|last| last.label == block.label && last.chunk_id == block.chunk_id)
        {
            last.end_ms = block.end_ms.or(last.end_ms);
            if !block.text.trim().is_empty() {
                if !last.text.ends_with(' ') && !last.text.is_empty() {
                    last.text.push(' ');
                }
                last.text.push_str(block.text.trim());
            }
        } else {
            merged.push(block);
        }
    }
    merged
}

fn chunk_quality(pcm: &[f32], sample_rate: u32) -> ChunkQuality {
    let audio_duration_s = if sample_rate == 0 {
        0.0
    } else {
        pcm.len() as f32 / sample_rate as f32
    };
    let rms_energy = rms(pcm);
    let clipping = pcm.iter().any(|sample| sample.abs() >= CLIP_GATE);
    let mut frames = 0usize;
    let mut speech_frames = 0usize;
    for frame in pcm.chunks(VAD_FRAME_SAMPLES) {
        if frame.is_empty() {
            continue;
        }
        frames += 1;
        if rms(frame) >= SPEECH_RMS_GATE {
            speech_frames += 1;
        }
    }
    let vad_purity = if frames == 0 {
        0.0
    } else {
        speech_frames as f32 / frames as f32
    };
    ChunkQuality {
        audio_duration_s,
        vad_purity,
        rms_energy,
        clipping,
    }
}

fn local_speaker_label(index: usize) -> String {
    let mut n = index;
    let mut chars = Vec::new();
    loop {
        chars.push((b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    chars.reverse();
    format!("{LOCAL_SPEAKER_PREFIX}{}", chars.iter().collect::<String>())
}

fn samples_to_ms(samples: usize, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    ((samples as u128 * 1000) / sample_rate as u128) as u64
}

fn ms_to_samples(ms: u64, sample_rate: u32) -> usize {
    ((ms as u128 * sample_rate as u128) / 1000) as usize
}

pub(crate) fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let ma = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if ma > 0.0 && mb > 0.0 {
        dot / (ma * mb)
    } else {
        0.0
    }
}

fn l2_normalize(mut values: Vec<f32>) -> Vec<f32> {
    let mag = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if mag > 0.0 {
        values.iter_mut().for_each(|v| *v /= mag);
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CutReason;
    use std::collections::BTreeSet;

    fn cut(time_s: f32) -> SpeakerChangeCut {
        SpeakerChangeCut {
            time_s,
            end_s: time_s,
            score: 1.0,
            reasons: BTreeSet::from([CutReason::Pitch]),
        }
    }

    #[test]
    fn build_chunk_spans_uses_cuts() {
        let spans = build_chunk_spans(&[cut(3.0), cut(6.0), cut(9.0), cut(12.0)], 15_000);
        assert_eq!(
            spans,
            vec![
                ChunkSpan {
                    start_ms: 0,
                    end_ms: 3_000
                },
                ChunkSpan {
                    start_ms: 3_000,
                    end_ms: 6_000
                },
                ChunkSpan {
                    start_ms: 6_000,
                    end_ms: 9_000
                },
                ChunkSpan {
                    start_ms: 9_000,
                    end_ms: 12_000
                },
                ChunkSpan {
                    start_ms: 12_000,
                    end_ms: 15_000
                },
            ]
        );
    }

    #[test]
    fn build_chunk_spans_merges_tiny_spans() {
        let spans = build_chunk_spans(&[cut(0.5), cut(3.0), cut(3.5)], 5_000);
        assert_eq!(
            spans,
            vec![
                ChunkSpan {
                    start_ms: 0,
                    end_ms: 3_000
                },
                ChunkSpan {
                    start_ms: 3_000,
                    end_ms: 5_000
                },
            ]
        );
    }

    #[test]
    fn build_chunk_spans_keeps_near_two_second_first_turn() {
        let spans = build_chunk_spans(&[cut(1.86), cut(7.0)], 11_000);
        assert_eq!(
            spans,
            vec![
                ChunkSpan {
                    start_ms: 0,
                    end_ms: 2_000
                },
                ChunkSpan {
                    start_ms: 2_000,
                    end_ms: 7_000
                },
                ChunkSpan {
                    start_ms: 7_000,
                    end_ms: 11_000
                },
            ]
        );
    }

    #[test]
    fn block_labels_follow_parent_chunk() {
        let chunks = vec![
            test_chunk("chunk-0001", 0, 3_000, "Speaker A"),
            test_chunk("chunk-0002", 3_000, 6_000, "Speaker B"),
        ];
        let segments = vec![
            Segment::new(100, 900, "hello"),
            Segment::new(3_100, 3_900, "there"),
        ];
        let blocks = build_blocks_from_chunks(&segments, &chunks);
        assert_eq!(blocks[0].label, "Speaker A");
        assert_eq!(blocks[0].chunk_id.as_deref(), Some("chunk-0001"));
        assert_eq!(blocks[1].label, "Speaker B");
    }

    #[test]
    fn block_label_uses_largest_overlap_for_boundary_segments() {
        let chunks = vec![
            test_chunk("chunk-0001", 0, 7_670, "You"),
            test_chunk("chunk-0002", 7_670, 15_090, "Speaker B"),
        ];
        let segments = vec![Segment::new(5_000, 7_000, "volumes of forgotten lore")];
        let blocks = build_blocks_from_chunks(&segments, &chunks);
        assert_eq!(blocks[0].label, "You");
        assert_eq!(blocks[0].chunk_id.as_deref(), Some("chunk-0001"));
    }

    #[test]
    fn local_clustering_is_looser_than_profile_matching() {
        let chunks = vec![
            test_chunk_with_embedding("chunk-0001", 0, 3_000, vec![1.0, 0.0]),
            // Cosine with chunk 1 is 0.8: below profile threshold 0.9, above
            // local cluster threshold 0.6. This should stay the same session speaker.
            test_chunk_with_embedding("chunk-0002", 3_000, 6_000, vec![0.8, 0.6]),
            test_chunk_with_embedding("chunk-0003", 6_000, 9_000, vec![0.0, 1.0]),
        ];
        let clusters = cluster_chunks(&chunks, &[], 0.9);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].members, vec![0, 1]);
        assert_eq!(clusters[1].members, vec![2]);
    }

    #[test]
    fn session_speaker_centroid_prefers_long_clean_chunks() {
        let mut short = test_chunk_with_embedding("chunk-0001", 0, 2_000, vec![1.0, 0.0]);
        short.label = "Gilgamesh".to_string();
        let mut long = test_chunk_with_embedding("chunk-0002", 2_000, 17_000, vec![0.0, 1.0]);
        long.label = "Gilgamesh".to_string();

        let speakers = build_session_speakers(&[short, long]);
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].label, "Gilgamesh");
        assert_eq!(
            speakers[0].clean_chunk_ids,
            vec!["chunk-0001".to_string(), "chunk-0002".to_string()]
        );
        assert_eq!(speakers[0].duration_ms, 17_000);
        assert!(
            speakers[0].centroid_embedding[1] > speakers[0].centroid_embedding[0],
            "long clean chunk should carry more centroid weight: {:?}",
            speakers[0].centroid_embedding
        );
        assert!(!speakers[0].user_confirmed);
    }

    #[test]
    fn session_speaker_uses_only_clean_chunks() {
        let clean = test_chunk_with_embedding("chunk-0001", 0, 5_000, vec![1.0, 0.0]);
        let mut clipped = test_chunk_with_embedding("chunk-0002", 5_000, 12_000, vec![0.0, 1.0]);
        clipped.clipping = true;
        let mut low_purity =
            test_chunk_with_embedding("chunk-0003", 12_000, 20_000, vec![0.0, 1.0]);
        low_purity.vad_purity = 0.2;

        let speakers = build_session_speakers(&[clean, clipped, low_purity]);
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].clean_chunk_ids, vec!["chunk-0001"]);
        assert_eq!(speakers[0].duration_ms, 5_000);
    }

    #[test]
    fn score_chunks_backfills_session_score_and_margin() {
        let mut a1 = test_chunk_with_embedding("chunk-0001", 0, 10_000, vec![1.0, 0.0]);
        a1.cluster_id = Some("speaker-1".to_string());
        let mut a2 = test_chunk_with_embedding("chunk-0002", 10_000, 12_000, vec![0.6, 0.8]);
        a2.cluster_id = Some("speaker-1".to_string());
        let mut b1 = test_chunk_with_embedding("chunk-0003", 12_000, 20_000, vec![0.0, 1.0]);
        b1.cluster_id = Some("speaker-2".to_string());

        let mut chunks = vec![a1, a2, b1];
        let speakers = build_session_speakers(&chunks);
        assert_eq!(speakers.len(), 2);
        score_chunks(&mut chunks, &speakers);

        for chunk in &chunks {
            let own = speakers
                .iter()
                .find(|speaker| Some(&speaker.session_speaker_id) == chunk.cluster_id.as_ref())
                .expect("own session speaker");
            let embedding = chunk.embedding.as_deref().expect("embedding");
            let expected_score = cosine(embedding, &own.centroid_embedding);
            let score = chunk.session_score.expect("session_score set");
            assert!(
                (score - expected_score).abs() < 1e-6,
                "chunk {} session_score {score} != cosine to own centroid {expected_score}",
                chunk.id
            );

            let mut all: Vec<f32> = speakers
                .iter()
                .map(|speaker| cosine(embedding, &speaker.centroid_embedding))
                .collect();
            all.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let expected_margin = all[0] - all[1];
            let margin = chunk.margin.expect("margin set");
            assert!(
                (margin - expected_margin).abs() < 1e-6,
                "chunk {} margin {margin} != best-minus-second {expected_margin}",
                chunk.id
            );
        }
    }

    #[test]
    fn score_chunks_skips_chunks_without_embedding() {
        let embedded = test_chunk_with_embedding("chunk-0001", 0, 5_000, vec![1.0, 0.0]);
        let plain = test_chunk("chunk-0002", 5_000, 8_000, "Other");

        let mut chunks = vec![embedded, plain];
        let speakers = build_session_speakers(&chunks);
        score_chunks(&mut chunks, &speakers);

        assert!(chunks[1].session_score.is_none());
        assert!(chunks[1].margin.is_none());
    }

    #[test]
    fn score_chunks_needs_two_speakers_for_margin() {
        let mut only = test_chunk_with_embedding("chunk-0001", 0, 5_000, vec![1.0, 0.0]);
        only.cluster_id = Some("speaker-1".to_string());

        let mut chunks = vec![only];
        let speakers = build_session_speakers(&chunks);
        assert_eq!(speakers.len(), 1);
        score_chunks(&mut chunks, &speakers);

        assert!(chunks[0].session_score.is_some());
        assert!(
            chunks[0].margin.is_none(),
            "margin needs a second speaker to compare against"
        );
    }

    #[test]
    fn score_chunks_scores_margin_even_when_own_cluster_has_no_centroid() {
        // A clipped chunk is excluded from session speakers, so its own cluster
        // has no centroid — but it can still be measured against the others.
        let mut clean_a = test_chunk_with_embedding("chunk-0001", 0, 5_000, vec![1.0, 0.0]);
        clean_a.cluster_id = Some("speaker-1".to_string());
        let mut clean_b = test_chunk_with_embedding("chunk-0002", 5_000, 10_000, vec![0.0, 1.0]);
        clean_b.cluster_id = Some("speaker-2".to_string());
        let mut clipped = test_chunk_with_embedding("chunk-0003", 10_000, 14_000, vec![0.6, 0.8]);
        clipped.cluster_id = Some("speaker-3".to_string());
        clipped.clipping = true;

        let mut chunks = vec![clean_a, clean_b, clipped];
        let speakers = build_session_speakers(&chunks);
        assert_eq!(speakers.len(), 2);
        score_chunks(&mut chunks, &speakers);

        assert!(chunks[2].session_score.is_none());
        assert!(chunks[2].margin.is_some());
    }

    fn clustered_chunk(
        id: &str,
        start_ms: u64,
        end_ms: u64,
        embedding: Vec<f32>,
        cluster: &str,
        label: &str,
    ) -> SpeakerChunk {
        let mut chunk = test_chunk_with_embedding(id, start_ms, end_ms, embedding);
        chunk.cluster_id = Some(cluster.to_string());
        chunk.label = label.to_string();
        chunk
    }

    #[test]
    fn correction_moves_chunk_and_recomputes_both_centroids() {
        let mut chunks = vec![
            clustered_chunk(
                "chunk-0001",
                0,
                10_000,
                vec![1.0, 0.0],
                "speaker-1",
                "Speaker A",
            ),
            clustered_chunk(
                "chunk-0002",
                10_000,
                12_000,
                vec![0.6, 0.8],
                "speaker-1",
                "Speaker A",
            ),
            clustered_chunk(
                "chunk-0003",
                12_000,
                20_000,
                vec![0.0, 1.0],
                "speaker-2",
                "Speaker B",
            ),
        ];
        let mut speakers = build_session_speakers(&chunks);
        score_chunks(&mut chunks, &speakers);

        let outcome =
            correct_chunk_label(&mut chunks, &mut speakers, "chunk-0002", "Speaker B", 42)
                .expect("correction applies");

        assert_eq!(outcome.corrected_chunk_id.as_deref(), Some("chunk-0002"));
        let moved = &chunks[1];
        assert_eq!(moved.label, "Speaker B");
        assert_eq!(moved.cluster_id.as_deref(), Some("speaker-2"));
        assert_eq!(
            moved.corrections,
            vec![crate::types::LabelCorrection {
                from_label: "Speaker A".into(),
                to_label: "Speaker B".into(),
                corrected_at_ms: 42,
                auto: false,
            }]
        );

        // speaker-1 lost the member: centroid is chunk-0001 alone again.
        let a = speakers
            .iter()
            .find(|s| s.session_speaker_id == "speaker-1")
            .expect("speaker-1 remains");
        assert!((a.centroid_embedding[0] - 1.0).abs() < 1e-6);
        assert_eq!(a.clean_chunk_ids, vec!["chunk-0001"]);

        // speaker-2 gained it: centroid pulled off the [0,1] axis, confirmed.
        let b = speakers
            .iter()
            .find(|s| s.session_speaker_id == "speaker-2")
            .expect("speaker-2 remains");
        assert!(b.user_confirmed);
        assert!(b.centroid_embedding[0] > 0.0);
        assert_eq!(b.clean_chunk_ids.len(), 2);

        // Scores were refreshed against the updated centroids.
        let expected = cosine(
            chunks[1].embedding.as_deref().unwrap(),
            &b.centroid_embedding,
        );
        assert!((chunks[1].session_score.unwrap() - expected).abs() < 1e-6);
    }

    #[test]
    fn correction_to_unknown_label_creates_confirmed_session_speaker() {
        let mut chunks = vec![
            clustered_chunk(
                "chunk-0001",
                0,
                10_000,
                vec![1.0, 0.0],
                "speaker-1",
                "Speaker A",
            ),
            clustered_chunk(
                "chunk-0002",
                10_000,
                15_000,
                vec![0.9, 0.436],
                "speaker-1",
                "Speaker A",
            ),
        ];
        let mut speakers = build_session_speakers(&chunks);

        correct_chunk_label(&mut chunks, &mut speakers, "chunk-0002", "Alice", 7)
            .expect("correction applies");

        let alice = speakers
            .iter()
            .find(|s| s.label == "Alice")
            .expect("new session speaker created");
        assert!(alice.user_confirmed);
        assert_eq!(alice.clean_chunk_ids, vec!["chunk-0002"]);
        assert_ne!(alice.session_speaker_id, "speaker-1");
        assert_eq!(
            chunks[1].cluster_id.as_deref(),
            Some(alice.session_speaker_id.as_str())
        );
    }

    #[test]
    fn correction_cascades_to_clearly_misfiled_chunks() {
        // chunk-0003 and chunk-0004 both sound like Speaker A but sit in
        // cluster B. Correcting one should pull the other across too.
        let mut chunks = vec![
            clustered_chunk(
                "chunk-0001",
                0,
                10_000,
                vec![1.0, 0.0],
                "speaker-1",
                "Speaker A",
            ),
            clustered_chunk(
                "chunk-0002",
                10_000,
                20_000,
                vec![0.0, 1.0],
                "speaker-2",
                "Speaker B",
            ),
            clustered_chunk(
                "chunk-0003",
                20_000,
                22_000,
                vec![0.9, 0.436],
                "speaker-2",
                "Speaker B",
            ),
            clustered_chunk(
                "chunk-0004",
                22_000,
                24_000,
                vec![0.95, 0.312],
                "speaker-2",
                "Speaker B",
            ),
        ];
        let mut speakers = build_session_speakers(&chunks);
        score_chunks(&mut chunks, &speakers);

        let outcome = correct_chunk_label(&mut chunks, &mut speakers, "chunk-0003", "Speaker A", 9)
            .expect("correction applies");

        assert_eq!(
            outcome.auto_corrected_chunk_ids,
            vec!["chunk-0004".to_string()],
            "the other misfiled chunk should cascade"
        );
        assert_eq!(chunks[3].label, "Speaker A");
        assert_eq!(chunks[3].cluster_id.as_deref(), Some("speaker-1"));
        let auto = chunks[3]
            .corrections
            .last()
            .expect("auto correction recorded");
        assert!(auto.auto);
        assert_eq!(auto.from_label, "Speaker B");
        // The untouched anchors keep their homes.
        assert_eq!(chunks[0].cluster_id.as_deref(), Some("speaker-1"));
        assert_eq!(chunks[1].cluster_id.as_deref(), Some("speaker-2"));
        assert!(chunks[1].corrections.is_empty());
    }

    #[test]
    fn cascade_never_overrides_explicit_user_corrections() {
        let mut chunks = vec![
            clustered_chunk(
                "chunk-0001",
                0,
                10_000,
                vec![1.0, 0.0],
                "speaker-1",
                "Speaker A",
            ),
            clustered_chunk(
                "chunk-0002",
                10_000,
                20_000,
                vec![0.0, 1.0],
                "speaker-2",
                "Speaker B",
            ),
            // Sounds like A, but the user has already pinned it to B.
            clustered_chunk(
                "chunk-0003",
                20_000,
                22_000,
                vec![0.95, 0.312],
                "speaker-2",
                "Speaker B",
            ),
            clustered_chunk(
                "chunk-0004",
                22_000,
                24_000,
                vec![0.9, 0.436],
                "speaker-2",
                "Speaker B",
            ),
        ];
        chunks[2].corrections.push(crate::types::LabelCorrection {
            from_label: "Speaker A".into(),
            to_label: "Speaker B".into(),
            corrected_at_ms: 1,
            auto: false,
        });
        let mut speakers = build_session_speakers(&chunks);
        score_chunks(&mut chunks, &speakers);

        let outcome = correct_chunk_label(&mut chunks, &mut speakers, "chunk-0004", "Speaker A", 9)
            .expect("correction applies");

        assert!(outcome.auto_corrected_chunk_ids.is_empty());
        assert_eq!(chunks[2].label, "Speaker B", "user-pinned chunk must stay");
        assert_eq!(chunks[2].cluster_id.as_deref(), Some("speaker-2"));
    }

    #[test]
    fn cascade_skips_ambiguous_chunks_below_margin_gate() {
        // chunk-0003 sits between the two centroids (margin < 0.05): it must
        // not be auto-relabeled even though a winner nominally exists.
        let mut chunks = vec![
            clustered_chunk(
                "chunk-0001",
                0,
                10_000,
                vec![1.0, 0.0],
                "speaker-1",
                "Speaker A",
            ),
            clustered_chunk(
                "chunk-0002",
                10_000,
                20_000,
                vec![0.0, 1.0],
                "speaker-2",
                "Speaker B",
            ),
            clustered_chunk(
                "chunk-0003",
                20_000,
                22_000,
                vec![0.72, 0.694],
                "speaker-2",
                "Speaker B",
            ),
            clustered_chunk(
                "chunk-0004",
                22_000,
                24_000,
                vec![0.9, 0.436],
                "speaker-2",
                "Speaker B",
            ),
        ];
        let mut speakers = build_session_speakers(&chunks);
        score_chunks(&mut chunks, &speakers);

        correct_chunk_label(&mut chunks, &mut speakers, "chunk-0004", "Speaker A", 9)
            .expect("correction applies");

        let ambiguous = &chunks[2];
        let margin = ambiguous.margin.expect("margin scored");
        assert!(
            margin < 0.05,
            "test fixture must stay ambiguous, got margin {margin}"
        );
        assert_eq!(ambiguous.label, "Speaker B", "ambiguous chunk stays put");
        assert!(ambiguous.corrections.is_empty());
    }

    #[test]
    fn correction_to_same_label_is_a_noop() {
        let mut chunks = vec![clustered_chunk(
            "chunk-0001",
            0,
            10_000,
            vec![1.0, 0.0],
            "speaker-1",
            "Speaker A",
        )];
        let mut speakers = build_session_speakers(&chunks);

        let outcome = correct_chunk_label(&mut chunks, &mut speakers, "chunk-0001", "Speaker A", 5)
            .expect("noop allowed");

        assert_eq!(outcome, CorrectionOutcome::default());
        assert!(chunks[0].corrections.is_empty());
    }

    #[test]
    fn correction_unknown_chunk_errors() {
        let mut chunks: Vec<SpeakerChunk> = Vec::new();
        let mut speakers: Vec<SessionSpeaker> = Vec::new();
        assert!(
            correct_chunk_label(&mut chunks, &mut speakers, "chunk-9999", "Speaker A", 5).is_err()
        );
    }

    #[test]
    fn local_labels_continue_after_z() {
        assert_eq!(local_speaker_label(0), "Speaker A");
        assert_eq!(local_speaker_label(25), "Speaker Z");
        assert_eq!(local_speaker_label(26), "Speaker AA");
    }

    fn test_chunk(id: &str, start_ms: u64, end_ms: u64, label: &str) -> SpeakerChunk {
        SpeakerChunk {
            id: id.to_string(),
            start_ms,
            end_ms,
            label: label.to_string(),
            cluster_id: Some("speaker-1".to_string()),
            matched_profile: None,
            embedding: None,
            encrypted_embedding: None,
            audio_duration_s: 3.0,
            vad_purity: 1.0,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: None,
            margin: None,
            corrections: Vec::new(),
        }
    }

    fn test_chunk_with_embedding(
        id: &str,
        start_ms: u64,
        end_ms: u64,
        embedding: Vec<f32>,
    ) -> SpeakerChunk {
        let mut chunk = test_chunk(id, start_ms, end_ms, "Other");
        chunk.embedding = Some(l2_normalize(embedding));
        chunk
    }
}
