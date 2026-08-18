//! Align timed ASR segments to anonymous diarization ranges.
//!
//! Pure functions, no I/O: given Whisper's `Segment`s and Sortformer's
//! [`DiarizationRange`]s, produce display-ready `SpeakerBlock`s labeled
//! "Speaker 1".."Speaker 4". A segment takes the label of the speaker with the
//! **maximum summed time overlap**; no overlap at all → [`UNKNOWN_SPEAKER_LABEL`].
//! Adjacent same-label blocks merge (single space join) via
//! `speaker_blocks::merge_blocks`.

use crate::services::speaker_blocks::merge_blocks;
use crate::types::{DiarizationRange, Segment, SpeakerBlock};

/// Label for un-diarized time; matches the existing display convention so the
/// frontend needs no changes.
pub const UNKNOWN_SPEAKER_LABEL: &str = "Other";

/// Display label for a Sortformer slot: 0 → "Speaker 1".
pub fn speaker_label(speaker_id: u8) -> String {
    format!("Speaker {}", u16::from(speaker_id) + 1)
}

/// Max-overlap assignment of segments to diarization ranges.
///
/// Overlap is summed per `speaker_id` across all of that speaker's ranges, so a
/// speaker whose activity is split into many spans still wins a segment they
/// dominate. Ties break toward the lower `speaker_id` (deterministic regardless
/// of range order). Whitespace-only segments are skipped.
pub fn align_ranges_to_segments(
    segments: &mut [Segment],
    ranges: &[DiarizationRange],
) -> Vec<SpeakerBlock> {
    for segment in segments.iter_mut() {
        segment.speaker = Some(label_for_segment(segment, ranges));
    }
    let blocks = segments
        .iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .map(|segment| {
            crate::services::speaker_blocks::block_from_segment(
                segment,
                segment.speaker.as_deref().unwrap_or(UNKNOWN_SPEAKER_LABEL),
            )
        })
        .collect();
    merge_blocks(blocks)
}

fn label_for_segment(segment: &Segment, ranges: &[DiarizationRange]) -> String {
    let seg_start = segment.start_ms.max(0) as u64;
    let seg_end = segment.end_ms.max(0) as u64;
    // Summed overlap per speaker slot; BTreeMap iterates ascending speaker_id,
    // so keeping only strictly-greater sums makes ties land on the lower id.
    let mut overlap_by_speaker = std::collections::BTreeMap::<u8, u64>::new();
    for range in ranges {
        let overlap = seg_end
            .min(range.end_ms)
            .saturating_sub(seg_start.max(range.start_ms));
        if overlap > 0 {
            *overlap_by_speaker.entry(range.speaker_id).or_default() += overlap;
        }
    }
    let mut best: Option<(u8, u64)> = None;
    for (speaker_id, overlap) in overlap_by_speaker {
        if best.is_none_or(|(_, top)| overlap > top) {
            best = Some((speaker_id, overlap));
        }
    }
    match best {
        Some((speaker_id, _)) => speaker_label(speaker_id),
        None => UNKNOWN_SPEAKER_LABEL.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start_ms: i64, end_ms: i64, text: &str) -> Segment {
        Segment::new(start_ms, end_ms, text)
    }

    fn range(speaker_id: u8, start_ms: u64, end_ms: u64) -> DiarizationRange {
        DiarizationRange {
            speaker_id,
            start_ms,
            end_ms,
        }
    }

    fn labels(blocks: &[SpeakerBlock]) -> Vec<&str> {
        blocks.iter().map(|b| b.label.as_str()).collect()
    }

    fn speakers(segments: &[Segment]) -> Vec<Option<&str>> {
        segments.iter().map(|s| s.speaker.as_deref()).collect()
    }

    #[test]
    fn speaker_label_is_one_based() {
        assert_eq!(speaker_label(0), "Speaker 1");
        assert_eq!(speaker_label(3), "Speaker 4");
    }

    #[test]
    fn empty_ranges_yield_single_merged_other_block() {
        let mut segments = [seg(0, 1_000, "Hello."), seg(1_000, 2_000, "World.")];
        let blocks = align_ranges_to_segments(&mut segments, &[]);
        assert_eq!(labels(&blocks), vec![UNKNOWN_SPEAKER_LABEL]);
        assert_eq!(blocks[0].text, "Hello. World.");
        assert_eq!(blocks[0].start_ms, Some(0));
        assert_eq!(blocks[0].end_ms, Some(2_000));
        assert_eq!(
            speakers(&segments),
            vec![Some(UNKNOWN_SPEAKER_LABEL), Some(UNKNOWN_SPEAKER_LABEL)]
        );
    }

    #[test]
    fn single_covering_range_labels_everything_speaker_one() {
        let mut segments = [seg(0, 1_000, "Hello."), seg(1_000, 2_000, "World.")];
        let blocks = align_ranges_to_segments(&mut segments, &[range(0, 0, 2_000)]);
        assert_eq!(labels(&blocks), vec!["Speaker 1"]);
        assert_eq!(blocks[0].text, "Hello. World.");
        assert_eq!(speakers(&segments), vec![Some("Speaker 1"), Some("Speaker 1")]);
    }

    #[test]
    fn straddling_segment_takes_max_overlap_speaker() {
        // Segment 0-1000: 400 ms of speaker 1, 600 ms of speaker 2.
        let ranges = [range(0, 0, 400), range(1, 400, 1_000)];
        let mut segments = [seg(0, 1_000, "Mostly two.")];
        let blocks = align_ranges_to_segments(&mut segments, &ranges);
        assert_eq!(labels(&blocks), vec!["Speaker 2"]);
        assert_eq!(speakers(&segments), vec![Some("Speaker 2")]);
    }

    #[test]
    fn exact_tie_breaks_toward_lower_speaker_id() {
        let ranges = [range(1, 500, 1_000), range(0, 0, 500)];
        let mut segments = [seg(0, 1_000, "Even split.")];
        let blocks = align_ranges_to_segments(&mut segments, &ranges);
        assert_eq!(labels(&blocks), vec!["Speaker 1"]);
        assert_eq!(speakers(&segments), vec![Some("Speaker 1")]);
    }

    #[test]
    fn overlap_sums_across_a_speakers_scattered_ranges() {
        // Speaker 1 has 300+300=600 ms inside the segment; speaker 2 has 400 ms.
        let ranges = [
            range(0, 0, 300),
            range(1, 300, 700),
            range(0, 700, 1_000),
        ];
        let mut segments = [seg(0, 1_000, "Scattered.")];
        let blocks = align_ranges_to_segments(&mut segments, &ranges);
        assert_eq!(labels(&blocks), vec!["Speaker 1"]);
        assert_eq!(speakers(&segments), vec![Some("Speaker 1")]);
    }

    #[test]
    fn zero_overlap_segment_is_other() {
        let ranges = [range(0, 5_000, 6_000)];
        let mut segments = [seg(0, 1_000, "Before any speech."), seg(5_000, 6_000, "Covered.")];
        let blocks = align_ranges_to_segments(&mut segments, &ranges);
        assert_eq!(labels(&blocks), vec![UNKNOWN_SPEAKER_LABEL, "Speaker 1"]);
        assert_eq!(
            speakers(&segments),
            vec![Some(UNKNOWN_SPEAKER_LABEL), Some("Speaker 1")]
        );
    }

    #[test]
    fn interleaved_speakers_produce_alternating_blocks_and_adjacent_merge() {
        let ranges = [range(0, 0, 2_000), range(1, 2_000, 3_000), range(0, 3_000, 4_000)];
        let mut segments = [
            seg(0, 1_000, "A one."),
            seg(1_000, 2_000, "A two."),
            seg(2_000, 3_000, "B one."),
            seg(3_000, 4_000, "A three."),
        ];
        let blocks = align_ranges_to_segments(&mut segments, &ranges);
        assert_eq!(labels(&blocks), vec!["Speaker 1", "Speaker 2", "Speaker 1"]);
        assert_eq!(blocks[0].text, "A one. A two.");
        assert_eq!(
            speakers(&segments),
            vec![
                Some("Speaker 1"),
                Some("Speaker 1"),
                Some("Speaker 2"),
                Some("Speaker 1")
            ]
        );
    }

    #[test]
    fn whitespace_only_segments_are_skipped() {
        let mut segments = [seg(0, 500, "   "), seg(500, 1_000, "Real.")];
        let blocks = align_ranges_to_segments(&mut segments, &[range(2, 0, 1_000)]);
        assert_eq!(labels(&blocks), vec!["Speaker 3"]);
        assert_eq!(blocks[0].text, "Real.");
        assert_eq!(speakers(&segments), vec![Some("Speaker 3"), Some("Speaker 3")]);
    }

    #[test]
    fn negative_segment_times_clamp_to_zero() {
        let mut segments = [seg(-200, 500, "Early.")];
        let blocks = align_ranges_to_segments(&mut segments, &[range(0, 0, 500)]);
        assert_eq!(blocks[0].start_ms, Some(0));
        assert_eq!(labels(&blocks), vec!["Speaker 1"]);
        assert_eq!(speakers(&segments), vec![Some("Speaker 1")]);
    }

    #[test]
    fn unsorted_ranges_give_same_result_as_sorted() {
        let sorted = [range(0, 0, 400), range(1, 400, 1_000)];
        let unsorted = [range(1, 400, 1_000), range(0, 0, 400)];
        let mut from_sorted = [seg(0, 1_000, "Order free.")];
        let mut from_unsorted = [seg(0, 1_000, "Order free.")];
        assert_eq!(
            labels(&align_ranges_to_segments(&mut from_sorted, &sorted)),
            labels(&align_ranges_to_segments(&mut from_unsorted, &unsorted)),
        );
        assert_eq!(speakers(&from_sorted), speakers(&from_unsorted));
    }
}
