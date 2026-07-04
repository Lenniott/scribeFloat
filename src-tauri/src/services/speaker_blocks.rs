use crate::services::voiceprint::{merge_blocks, VoiceprintService};
use crate::types::{Segment, SegmentSource, SpeakerBlock, CHANNEL_LABEL_IN, CHANNEL_LABEL_OUT};

const IDENTITY_LABEL_OTHER: &str = "Other";

fn identify_mic_segment(
    segment: &Segment,
    session_pcm: &[f32],
    sample_rate: u32,
    voiceprint_svc: &VoiceprintService,
    profiles: &[crate::types::VoiceprintProfile],
    threshold: f32,
) -> String {
    let start_ms = segment.start_ms.max(0) as u64;
    let end_ms = segment.end_ms.max(segment.start_ms).max(0) as u64;
    let start = ((start_ms as u128 * sample_rate as u128) / 1000) as usize;
    let end = ((end_ms as u128 * sample_rate as u128) / 1000) as usize;
    let slice = session_pcm.get(start.min(session_pcm.len())..end.min(session_pcm.len()));
    match slice {
        Some(pcm) if pcm.len() >= sample_rate as usize * 2 => {
            match voiceprint_svc.embed(pcm, sample_rate) {
                Ok(embedding) => {
                    voiceprint_svc.identify_with_threshold(&embedding, profiles, threshold)
                }
                Err(err) => {
                    tracing::debug!(error = %err, "embed failed for segment, labelling Other");
                    IDENTITY_LABEL_OTHER.to_string()
                }
            }
        }
        _ => IDENTITY_LABEL_OTHER.to_string(),
    }
}

fn channel_label(source: Option<SegmentSource>) -> &'static str {
    match source {
        Some(SegmentSource::Speaker) => CHANNEL_LABEL_OUT,
        Some(SegmentSource::Mic) | None => CHANNEL_LABEL_IN,
    }
}

fn merge_blocks_same_label(blocks: Vec<SpeakerBlock>) -> Vec<SpeakerBlock> {
    merge_blocks(blocks)
}

fn merge_blocks_same_label_and_source(
    blocks: Vec<(Option<SegmentSource>, SpeakerBlock)>,
) -> Vec<SpeakerBlock> {
    let mut merged: Vec<(Option<SegmentSource>, SpeakerBlock)> = Vec::new();
    for (source, block) in blocks {
        if let Some((last_source, last)) = merged.last_mut() {
            if last.label == block.label && *last_source == source {
                last.end_ms = block.end_ms.or(last.end_ms);
                if !block.text.trim().is_empty() {
                    if !last.text.ends_with(' ') && !last.text.is_empty() {
                        last.text.push(' ');
                    }
                    last.text.push_str(block.text.trim());
                }
                continue;
            }
        }
        merged.push((source, block));
    }
    merged.into_iter().map(|(_, block)| block).collect()
}

fn block_from_segment(segment: &Segment, label: &str) -> SpeakerBlock {
    SpeakerBlock {
        label: label.to_string(),
        start_ms: Some(segment.start_ms.max(0) as u64),
        end_ms: Some(segment.end_ms.max(0) as u64),
        text: segment.text.trim().to_string(),
    }
}

/// Build speaker blocks for all three display tiers.
pub fn build_speaker_blocks(
    segments: &[Segment],
    session_pcm: &[f32],
    sample_rate: u32,
    voiceprint_svc: &VoiceprintService,
    threshold: f32,
    dual_source: bool,
) -> Result<Vec<SpeakerBlock>, anyhow::Error> {
    let profiles = voiceprint_svc.load_profiles()?;

    if !dual_source {
        if profiles.is_empty() {
            return Ok(Vec::new());
        }
        let mut blocks = Vec::new();
        for segment in segments {
            let text = segment.text.trim();
            if text.is_empty() {
                continue;
            }
            let label = identify_mic_segment(
                segment,
                session_pcm,
                sample_rate,
                voiceprint_svc,
                &profiles,
                threshold,
            );
            blocks.push(block_from_segment(segment, &label));
        }
        return Ok(merge_blocks_same_label(blocks));
    }

    if profiles.is_empty() {
        let mut blocks = Vec::new();
        for segment in segments {
            let text = segment.text.trim();
            if text.is_empty() {
                continue;
            }
            blocks.push(block_from_segment(segment, channel_label(segment.source)));
        }
        return Ok(merge_blocks_same_label(blocks));
    }

    let mut blocks = Vec::new();
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        let label = match segment.source {
            Some(SegmentSource::Speaker) => IDENTITY_LABEL_OTHER.to_string(),
            Some(SegmentSource::Mic) | None => identify_mic_segment(
                segment,
                session_pcm,
                sample_rate,
                voiceprint_svc,
                &profiles,
                threshold,
            ),
        };
        blocks.push((segment.source, block_from_segment(segment, &label)));
    }
    Ok(merge_blocks_same_label_and_source(blocks))
}

pub fn display_block_label(stored: &str, input_label: &str, output_label: &str) -> String {
    match stored {
        CHANNEL_LABEL_IN => input_label.to_string(),
        CHANNEL_LABEL_OUT => output_label.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Segment;

    fn temp_voiceprint_svc() -> crate::services::voiceprint::VoiceprintService {
        let dir = std::env::temp_dir().join(format!("vp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        crate::services::voiceprint::VoiceprintService::new(
            &dir.join("model.onnx"),
            &dir.join("profiles"),
            0.7,
        )
        .expect("voiceprint svc")
    }

    #[test]
    fn build_tier1_single_source_no_profiles_returns_empty() {
        let svc = temp_voiceprint_svc();
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 1_000,
            text: "plain text".to_string(),
            source: None,
        }];
        let blocks = build_speaker_blocks(&segments, &[], 16_000, &svc, 0.7, false).unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn build_tier2_dual_source_no_profiles_uses_in_out_labels() {
        let svc = temp_voiceprint_svc();
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "mic line".to_string(),
                source: Some(SegmentSource::Mic),
            },
            Segment {
                start_ms: 1_200,
                end_ms: 2_000,
                text: "speaker line".to_string(),
                source: Some(SegmentSource::Speaker),
            },
        ];
        let blocks = build_speaker_blocks(&segments, &[], 16_000, &svc, 0.7, true).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].label, CHANNEL_LABEL_IN);
        assert_eq!(blocks[1].label, CHANNEL_LABEL_OUT);
    }

    #[test]
    fn build_tier3_dual_source_loopback_always_other() {
        let svc = temp_voiceprint_svc();
        let profile = svc
            .new_profile("Alice", None, vec![1.0; 192])
            .expect("profile");
        svc.save_profile(&profile).expect("save profile");

        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "ad copy".to_string(),
                source: Some(SegmentSource::Speaker),
            },
            Segment {
                start_ms: 1_200,
                end_ms: 2_000,
                text: "mic line".to_string(),
                source: Some(SegmentSource::Mic),
            },
        ];

        let blocks = build_speaker_blocks(&segments, &[], 16_000, &svc, 0.7, true).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].label, IDENTITY_LABEL_OTHER);
        assert_eq!(blocks[0].text, "ad copy");
        assert_eq!(blocks[1].label, IDENTITY_LABEL_OTHER);
    }

    #[test]
    fn adjacent_in_out_segments_never_merge() {
        let svc = temp_voiceprint_svc();
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 500,
                text: "mic".to_string(),
                source: Some(SegmentSource::Mic),
            },
            Segment {
                start_ms: 600,
                end_ms: 1_000,
                text: "speaker".to_string(),
                source: Some(SegmentSource::Speaker),
            },
        ];
        let blocks = build_speaker_blocks(&segments, &[], 16_000, &svc, 0.7, true).unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn display_block_label_maps_in_out_to_config() {
        assert_eq!(
            display_block_label(CHANNEL_LABEL_IN, "Mic", "Speaker"),
            "Mic"
        );
        assert_eq!(
            display_block_label(CHANNEL_LABEL_OUT, "Mic", "Speaker"),
            "Speaker"
        );
        assert_eq!(display_block_label("You", "Mic", "Speaker"), "You");
    }
}
