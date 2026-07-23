use crate::types::{Segment, SegmentSource, SpeakerBlock, CHANNEL_LABEL_IN, CHANNEL_LABEL_OUT};

/// Merge adjacent blocks sharing a label: extend the end time, join text with a
/// single space. Skips whitespace-only fragments.
pub fn merge_blocks(blocks: Vec<SpeakerBlock>) -> Vec<SpeakerBlock> {
    let mut merged: Vec<SpeakerBlock> = Vec::new();
    for block in blocks {
        if let Some(last) = merged.last_mut().filter(|last| last.label == block.label) {
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

fn channel_label(source: Option<SegmentSource>) -> &'static str {
    match source {
        Some(SegmentSource::Speaker) => CHANNEL_LABEL_OUT,
        Some(SegmentSource::Mic) | None => CHANNEL_LABEL_IN,
    }
}

pub(crate) fn block_from_segment(segment: &Segment, label: &str) -> SpeakerBlock {
    SpeakerBlock {
        label: label.to_string(),
        start_ms: Some(segment.start_ms.max(0) as u64),
        end_ms: Some(segment.end_ms.max(0) as u64),
        text: segment.text.trim().to_string(),
        chunk_id: None,
    }
}

/// Channel-tier blocks for dual-source captures: label purely by which track a
/// segment came from (`In` = mic, `Out` = loopback), merging adjacent
/// same-channel runs. No identity, no models.
pub fn build_channel_blocks(segments: &[Segment]) -> Vec<SpeakerBlock> {
    let blocks = segments
        .iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .map(|segment| block_from_segment(segment, channel_label(segment.source)))
        .collect();
    merge_blocks(blocks)
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

    fn sourced(start_ms: i64, end_ms: i64, text: &str, source: Option<SegmentSource>) -> Segment {
        let mut segment = Segment::new(start_ms, end_ms, text);
        segment.source = source;
        segment
    }

    #[test]
    fn build_channel_blocks_labels_by_track_and_merges_adjacent() {
        let segments = [
            sourced(0, 500, "me one", Some(SegmentSource::Mic)),
            sourced(500, 900, "me two", Some(SegmentSource::Mic)),
            sourced(900, 1_500, "them", Some(SegmentSource::Speaker)),
            sourced(1_500, 2_000, "me again", None),
        ];
        let blocks = build_channel_blocks(&segments);
        let labels: Vec<&str> = blocks.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, vec![CHANNEL_LABEL_IN, CHANNEL_LABEL_OUT, CHANNEL_LABEL_IN]);
        assert_eq!(blocks[0].text, "me one me two");
    }

    #[test]
    fn build_channel_blocks_skips_whitespace_segments() {
        let segments = [
            sourced(0, 500, "  ", Some(SegmentSource::Mic)),
            sourced(500, 900, "real", Some(SegmentSource::Speaker)),
        ];
        let blocks = build_channel_blocks(&segments);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].label, CHANNEL_LABEL_OUT);
    }
    #[test]
    fn adjacent_in_out_segments_never_merge() {
        let segments = [
            sourced(0, 500, "mic", Some(SegmentSource::Mic)),
            sourced(600, 1_000, "speaker", Some(SegmentSource::Speaker)),
        ];
        let blocks = build_channel_blocks(&segments);
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
