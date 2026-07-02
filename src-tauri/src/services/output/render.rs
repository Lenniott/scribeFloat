use crate::types::{Note, ReplacementRule, ReplacementScope, Segment, SegmentSource, SpeakerBlock};
use crate::services::speaker_blocks::display_block_label;

use super::dedup::{dedup_consecutive_phrases, dedup_repeated_block};
use super::cleanup::cleanup_text;
use super::replacements::apply_replacements;

fn segment_channel_key(source: Option<SegmentSource>) -> &'static str {
    match source {
        Some(SegmentSource::Mic) => "in",
        Some(SegmentSource::Speaker) => "out",
        None => "",
    }
}

fn format_ms(ms: i64) -> String {
    let total = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

/// Group segments into paragraphs and render the transcript body (after replacement rules).
/// Consecutive same-source segments separated by less than 8 s merge into one paragraph;
/// mic and speaker sources never merge. Shared by the markdown writer,
/// on-demand export/preview, and word counting.
pub fn render_transcript_body(
    segments: &[Segment],
    include_timestamps: bool,
    rules: &[ReplacementRule],
    prefix: &str,
) -> String {
    const MERGE_GAP_MS: i64 = 8_000;
    struct Group {
        start_ms: i64,
        end_ms: i64,
        parts: Vec<String>,
        source: &'static str,
    }
    let mut groups: Vec<Group> = Vec::new();
    for seg in segments {
        let clean = cleanup_text(seg.text.trim());
        if clean.is_empty() {
            continue;
        }
        let deduped = dedup_repeated_block(&dedup_consecutive_phrases(&clean));
        let seg_source = segment_channel_key(seg.source);
        let last = groups
            .last_mut()
            .filter(|g| seg.start_ms - g.end_ms < MERGE_GAP_MS && g.source == seg_source);
        match last {
            Some(g) => {
                g.end_ms = seg.end_ms;
                g.parts.push(deduped);
            }
            None => groups.push(Group {
                start_ms: seg.start_ms,
                end_ms: seg.end_ms,
                parts: vec![deduped],
                source: seg_source,
            }),
        }
    }
    let raw_body = {
        let mut out = String::new();
        for (i, g) in groups.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n");
            }
            let text = g.parts.join(" ");
            if include_timestamps {
                out.push_str(&format!("[{}] {}", format_ms(g.start_ms), text));
            } else {
                out.push_str(&text);
            }
        }
        out
    };
    apply_replacements(&raw_body, rules, &ReplacementScope::Transcripts, prefix)
}

/// Count words in the rendered transcript body, excluding timestamp labels.
pub fn count_words(segments: &[Segment], rules: &[ReplacementRule], prefix: &str) -> usize {
    render_transcript_body(segments, false, rules, prefix)
        .split_whitespace()
        .count()
}

fn format_speaker_time(ms: Option<u64>) -> Option<String> {
    ms.map(|value| {
        let total = value / 1000;
        format!("{:02}:{:02}", total / 60, total % 60)
    })
}

pub fn render_speaker_blocks_body(
    blocks: &[SpeakerBlock],
    rules: &[ReplacementRule],
    prefix: &str,
    input_label: &str,
    output_label: &str,
) -> String {
    let mut out = String::new();
    for (index, block) in blocks.iter().enumerate() {
        let cleaned = cleanup_text(block.text.trim());
        if cleaned.is_empty() {
            continue;
        }
        let deduped = dedup_repeated_block(&dedup_consecutive_phrases(&cleaned));
        if index > 0 {
            out.push_str("\n\n---\n\n");
        }
        let label = display_block_label(&block.label, input_label, output_label);
        out.push_str(&format!("**[{label}]**"));
        if let (Some(start), Some(end)) = (
            format_speaker_time(block.start_ms),
            format_speaker_time(block.end_ms),
        ) {
            out.push_str(&format!(" · {start}–{end}"));
        }
        out.push_str("\n\n");
        out.push_str(&deduped);
    }
    apply_replacements(&out, rules, &ReplacementScope::Transcripts, prefix)
}

/// Render a complete transcript markdown document (YAML front matter + `## Transcript` +
/// optional `## Notes`, trailing newline). Pure: performs no file I/O.
pub fn render_transcript_markdown(
    segments: &[Segment],
    notes: &[Note],
    title: &str,
    model_name: &str,
    include_timestamps: bool,
    rules: &[ReplacementRule],
    prefix: &str,
) -> String {
    let transcript_body = render_transcript_body(segments, include_timestamps, rules, prefix);

    let duration_seconds = segments
        .last()
        .map(|s| s.end_ms.max(0) as f64 / 1000.0)
        .unwrap_or(0.0);
    let word_count = count_words(segments, rules, prefix);
    let token_estimate = ((word_count as f64) * 1.3).round() as usize;

    let mut md = String::new();
    md.push_str("---\n");
    md.push_str(&format!("title: '{}'\n", title.replace('\'', "''")));
    md.push_str(&format!("duration_seconds: {:.1}\n", duration_seconds));
    md.push_str(&format!("word_count: {word_count}\n"));
    md.push_str(&format!("token_estimate: {token_estimate}\n"));
    md.push_str(&format!("model: {model_name}\n"));
    md.push_str("---\n\n");
    md.push_str("## Transcript\n\n");
    md.push_str(&transcript_body);

    if !notes.is_empty() {
        md.push_str("\n\n## Notes\n");
        for (i, note) in notes.iter().enumerate() {
            md.push_str(&format!(
                "[{}] ({}) {}\n",
                i + 1,
                format_ms(note.recorded_at_ms as i64),
                note.text
            ));
        }
    }

    md.push('\n');
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Note, ReplacementRule, ReplacementRuleType, ReplacementScope, WordTransform};

    fn simple_rule(trigger: &str, output: &str, scope: ReplacementScope) -> ReplacementRule {
        ReplacementRule {
            trigger: trigger.to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Simple,
            output: output.to_string(),
            scope,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        }
    }

    #[test]
    fn format_ms_zero() {
        assert_eq!(format_ms(0), "00:00:00");
    }

    #[test]
    fn format_ms_seconds_only() {
        assert_eq!(format_ms(12_000), "00:00:12");
    }

    #[test]
    fn format_ms_minutes_and_seconds() {
        assert_eq!(format_ms(90_000), "00:01:30");
    }

    #[test]
    fn format_ms_hours_minutes_seconds() {
        assert_eq!(format_ms(3_661_000), "01:01:01");
    }

    #[test]
    fn format_ms_pads_single_digits() {
        assert_eq!(format_ms(3_600_000 + 60_000 + 5_000), "01:01:05");
    }

    #[test]
    fn count_words_excludes_timestamp_labels() {
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
            source: None,
        }];
        assert_eq!(count_words(&segments, &[], ""), 2);
    }

    #[test]
    fn render_speaker_blocks_body_applies_cleanup_and_dedup() {
        use crate::types::SpeakerBlock;
        let blocks = vec![SpeakerBlock {
            label: "input".to_string(),
            start_ms: Some(0),
            end_ms: Some(5_000),
            text: "[MUSIC] hello hello world".to_string(),
        }];
        let body = render_speaker_blocks_body(&blocks, &[], "", "Input", "Output");
        assert!(
            body.contains("hello world"),
            "expected deduped text, got: {body}"
        );
        assert!(
            !body.contains("[MUSIC]"),
            "bracket annotations should be stripped, got: {body}"
        );
    }

    #[test]
    fn render_transcript_markdown_golden_dual_source_notes_rules_timestamps() {
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "hello dash world".to_string(),
                source: Some(SegmentSource::Mic),
            },
            Segment {
                start_ms: 1_200,
                end_ms: 3_000,
                text: "How are you?".to_string(),
                source: Some(SegmentSource::Speaker),
            },
            Segment {
                start_ms: 3_100,
                end_ms: 4_000,
                text: "I am well.".to_string(),
                source: Some(SegmentSource::Speaker),
            },
        ];
        let notes = vec![Note {
            id: "n1".to_string(),
            text: "follow up".to_string(),
            recorded_at_ms: 2_000,
        }];
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        let md =
            render_transcript_markdown(&segments, &notes, "My Title", "tiny", true, &rules, "");
        let expected = "---\n\
title: 'My Title'\n\
duration_seconds: 4.0\n\
word_count: 9\n\
token_estimate: 12\n\
model: tiny\n\
---\n\n\
## Transcript\n\n\
[00:00:00] hello - world\n\n\
[00:00:01] How are you? I am well.\n\n\
## Notes\n\
[1] (00:00:02) follow up\n\n";
        assert_eq!(md, expected);
    }
}
