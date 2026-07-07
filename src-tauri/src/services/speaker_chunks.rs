use crate::services::analysis::rms;
use crate::services::voiceprint::VoiceprintService;
use crate::types::{
    Segment, SessionSpeaker, SpeakerBlock, SpeakerChangeCut, SpeakerChunk, VoiceprintProfile,
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
            let centroid_embedding = weighted_centroid(&members)?;
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
            let radius = speaker_radius(&centroid_embedding, &members);
            let quality_score = speaker_quality_score(&members, radius);
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
        })
        .collect()
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

fn cosine(a: &[f32], b: &[f32]) -> f32 {
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
