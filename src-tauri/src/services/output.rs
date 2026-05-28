use crate::types::{
    DictateHistoryEntry, Note, ReplacementRule, ReplacementRuleType, ReplacementScope, Segment,
    WordTransform,
};
use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use regex::Regex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct OutputService;

#[derive(Debug, Serialize)]
struct SessionNotesPayload<'a> {
    format_version: u8,
    title: &'a str,
    wav_file: &'a str,
    notes: &'a [Note],
}

impl OutputService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    /// Create a timestamped session directory inside save_folder.
    pub fn make_session_dir(&self, save_folder: &str) -> Result<PathBuf> {
        let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let dir = PathBuf::from(save_folder).join(&ts);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Build the transcript file path using the recording title as the filename base.
    /// Spaces become underscores; chars forbidden on Windows/macOS become dashes.
    pub fn transcript_path(&self, session_dir: &Path, model_path: &Path, title: &str) -> PathBuf {
        let slug: String = title
            .chars()
            .map(|c| match c {
                ' ' => '_',
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                c => c,
            })
            .collect();
        let slug = if slug.is_empty() {
            chrono::Local::now()
                .format("%Y-%m-%d_%H-%M-%S")
                .to_string()
        } else {
            slug
        };
        let stem = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "model".to_string());
        session_dir.join(format!("{}_{}.md", slug, stem))
    }

    /// Write mono f32 PCM as a 16-bit WAV file.
    pub fn write_wav(&self, pcm: &[f32], sample_rate: u32, dest: &Path) -> Result<()> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(dest, spec).context("failed to create WAV writer")?;
        for &s in pcm {
            let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Join segments, clean Whisper artifacts, apply replacement rules, and return the final
    /// text ready for pasting. Scope applied: Dictate.
    pub fn format_dictate_text(&self, segments: &[Segment], rules: &[ReplacementRule]) -> String {
        let joined = segments
            .iter()
            .map(|s| cleanup_text(s.text.trim()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let deduped = dedup_repeated_block(&dedup_consecutive_phrases(&joined));
        apply_replacements(&deduped, rules, &ReplacementScope::Dictate)
    }

    /// Render segments as markdown and write. Verifies file is non-empty before returning Ok.
    #[allow(clippy::too_many_arguments)]
    pub fn write_transcript(
        &self,
        segments: &[Segment],
        notes: &[Note],
        title: &str,
        model_name: &str,
        include_timestamps: bool,
        rules: &[ReplacementRule],
        dest: &Path,
    ) -> Result<PathBuf> {
        // Merge consecutive same-source segments separated by less than 8 seconds into
        // a single paragraph. Never merge across speaker sources (in: vs out:).
        const MERGE_GAP_MS: i64 = 8_000;
        struct Group {
            start_ms: i64,
            end_ms: i64,
            parts: Vec<String>,
            source: &'static str, // "in", "out", or "" for single-source
        }
        let mut groups: Vec<Group> = Vec::new();
        for seg in segments {
            let clean = cleanup_text(seg.text.trim());
            if clean.is_empty() {
                continue;
            }
            let deduped = dedup_repeated_block(&dedup_consecutive_phrases(&clean));
            let seg_source = speaker_source_prefix(&deduped);
            let last = groups.last_mut().filter(|g| {
                seg.start_ms - g.end_ms < MERGE_GAP_MS && g.source == seg_source
            });
            match last {
                Some(g) => {
                    g.end_ms = seg.end_ms;
                    let body = deduped
                        .strip_prefix("in: ")
                        .or_else(|| deduped.strip_prefix("out: "))
                        .map(|s| s.to_string())
                        .unwrap_or(deduped);
                    g.parts.push(body);
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

        let transcript_body = apply_replacements(&raw_body, rules, &ReplacementScope::Transcripts);

        let duration_seconds = segments
            .last()
            .map(|s| s.end_ms.max(0) as f64 / 1000.0)
            .unwrap_or(0.0);
        let word_count = transcript_body.split_whitespace().count();
        let token_estimate = ((word_count as f64) * 1.3).round() as usize;

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("title: '{}'\n", title.replace('\'', "’")));
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
        std::fs::write(dest, &md).context("failed to write transcript")?;
        if std::fs::metadata(dest)?.len() == 0 {
            return Err(anyhow::anyhow!("transcript was written empty"));
        }
        Ok(dest.to_path_buf())
    }

    /// Delete a WAV file. Silent no-op if it no longer exists.
    pub fn delete_wav(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Write `[session_dir]/notes.json` capturing title and recorded notes for a WAV-only save.
    pub fn write_session_notes(
        &self,
        session_dir: &Path,
        title: &str,
        wav_file_name: &str,
        notes: &[Note],
    ) -> Result<PathBuf> {
        let payload = SessionNotesPayload {
            format_version: 1,
            title,
            wav_file: wav_file_name,
            notes,
        };
        let dest = session_dir.join("notes.json");
        let json =
            serde_json::to_string_pretty(&payload).context("failed to serialize notes.json")?;
        std::fs::write(&dest, json).context("failed to write notes.json")?;
        Ok(dest)
    }

    /// Read a transcript file. Path validation (boundary check) is the caller's responsibility;
    /// the actual I/O lives here to keep all disk access inside OutputService.
    pub fn read_transcript(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("failed to read transcript: {e}"))
    }

    /// Create the directory at `path` (and missing parents) and return its canonical form.
    /// Used when validating and persisting a new save-folder location.
    pub fn ensure_output_dir(&self, path: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("failed to create directory `{}`: {e}", path.display()))?;
        let canonical = std::fs::canonicalize(path)
            .map_err(|e| format!("failed to resolve path `{}`: {e}", path.display()))?;
        if !canonical.is_dir() {
            return Err(format!("`{}` is not a directory", canonical.display()));
        }
        Ok(canonical)
    }

    /// Open a file with the OS default handler (or a named app). Delegates to platform code
    /// so that controllers do not call the platform layer directly.
    pub fn open_file_for_user(&self, path: &str, app: Option<&str>) -> Result<(), String> {
        crate::platform::open_file(path, app)
    }

    /// Prepend a new entry to `{save_folder}/dictate_history.json`.
    /// Creates the save folder and file if they do not exist. The list is newest-first.
    pub fn write_dictate_history_entry(&self, save_folder: &str, text: &str) -> Result<()> {
        let path = PathBuf::from(save_folder).join("dictate_history.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create save folder for dictate history")?;
        }
        let mut entries: Vec<DictateHistoryEntry> = if path.exists() {
            let raw = std::fs::read_to_string(&path).context("read dictate_history.json")?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            Vec::new()
        };
        entries.insert(0, DictateHistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            text: text.to_string(),
        });
        let json = serde_json::to_string_pretty(&entries)
            .context("serialize dictate_history.json")?;
        std::fs::write(&path, json).context("write dictate_history.json")?;
        Ok(())
    }

    /// Read all entries from `{save_folder}/dictate_history.json` (newest-first).
    /// Returns an empty list if the file does not exist.
    pub fn read_dictate_history(&self, save_folder: &str) -> Result<Vec<DictateHistoryEntry>> {
        let path = PathBuf::from(save_folder).join("dictate_history.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&path).context("read dictate_history.json")?;
        serde_json::from_str(&raw).context("parse dictate_history.json")
    }

    /// Simulate Cmd/Ctrl+V into the currently focused application.
    /// Requires Accessibility permission on macOS. Caller must write text to clipboard first.
    pub fn paste_text(&self) -> Result<(), String> {
        crate::platform::paste_impl::paste_text()
    }

    /// Simulate pressing Enter in the currently focused application.
    pub fn send_enter(&self) -> Result<(), String> {
        crate::platform::paste_impl::send_enter()
    }

    /// Remove the session directory if it contains no files (i.e. recording was cancelled
    /// before any WAV was written). Silent no-op if the directory is non-empty or gone.
    pub fn delete_session_dir_if_empty(&self, dir: &Path) {
        if !dir.exists() {
            return;
        }
        let is_empty = std::fs::read_dir(dir)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

// ── Text cleanup ──────────────────────────────────────────────────────────────

/// Strip Whisper artifact annotations and normalize whitespace from a single segment.
/// Always-on — these are never valid speech output.
fn cleanup_text(text: &str) -> String {
    // Remove all-caps bracket annotations Whisper emits: [BLANK_AUDIO], [Music], [Applause], etc.
    // Pattern: literal [ followed by uppercase letter/underscore, then uppercase/space/underscore, then ]
    // Strip Whisper bracket annotations: first char uppercase, rest letters/space/underscore.
    // Matches [BLANK_AUDIO], [Music], [Sounds of the toilet] but not [ ] or [note].
    let annotation_re = Regex::new(r"\[[A-Z][A-Za-z_ ]*\]").expect("static regex");
    let cleaned = annotation_re.replace_all(text, "");
    // Whisper sometimes fuses its native "#word" output with a following command word
    // (e.g. "hashtag cake new line" → "#cakenewline"). Split so replacement rules fire.
    let fusion_re = Regex::new(r"(?i)(#\w+?)(newline)").expect("static regex");
    let cleaned = fusion_re.replace_all(&cleaned, "$1 $2");
    // Normalize internal whitespace
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    words.join(" ")
}

/// Remove consecutively repeated phrases of 1–5 words (case-insensitive).
/// Handles Whisper repetition artifacts at segment boundaries:
///   "hello world. world. Next"  → "hello world. Next"
///   "eat some food eat some food" → "eat some food"
fn dedup_consecutive_phrases(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < 2 {
        return text.to_string();
    }
    let mut result: Vec<&str> = Vec::with_capacity(tokens.len());
    result.push(tokens[0]);
    let mut i = 1;
    while i < tokens.len() {
        let mut skipped = false;
        // Longest match first so "eat some food" beats "eat" when both repeat
        for phrase_len in (1..=5).rev() {
            if i + phrase_len > tokens.len() || result.len() < phrase_len {
                continue;
            }
            let prev = &result[result.len() - phrase_len..];
            let curr = &tokens[i..i + phrase_len];
            let matches = prev.iter().zip(curr.iter()).all(|(p, c)| {
                let p = p.trim_end_matches(|ch: char| !ch.is_alphanumeric());
                let c = c.trim_end_matches(|ch: char| !ch.is_alphanumeric());
                p.eq_ignore_ascii_case(c)
            });
            if matches {
                i += phrase_len;
                skipped = true;
                break;
            }
        }
        if !skipped {
            result.push(tokens[i]);
            i += 1;
        }
    }
    result.join(" ")
}

/// If the transcript appears twice (Whisper hallucination on long audio), keep the first copy.
/// Uses the opening fingerprint (~20% of text, ≤100 chars) to detect the repeat start.
fn dedup_repeated_block(text: &str) -> String {
    let min_len = 60;
    if text.len() < min_len * 2 {
        return text.to_string();
    }
    let fp_len = (text.len() / 5).clamp(20, 100);
    let fingerprint = text[..fp_len].to_lowercase();
    // Search for the repeat only in the second half
    let search_from = text.len() / 2;
    if let Some(pos) = text[search_from..].to_lowercase().find(&fingerprint) {
        return text[..search_from + pos].trim_end().to_string();
    }
    text.to_string()
}

// ── Replacement engine ────────────────────────────────────────────────────────

/// Apply user-defined replacement rules to text. Rules are applied in order.
/// Only rules whose scope is Both or matches the given scope are applied.
fn apply_replacements(text: &str, rules: &[ReplacementRule], scope: &ReplacementScope) -> String {
    let mut result = text.to_string();
    for rule in rules {
        if rule.trigger.trim().is_empty() {
            continue;
        }
        let rule_scope = &rule.scope;
        if rule_scope != &ReplacementScope::Both && rule_scope != scope {
            continue;
        }
        let triggers = std::iter::once(rule.trigger.as_str())
            .chain(rule.aliases.iter().map(String::as_str))
            .filter(|t| !t.trim().is_empty());
        for trigger in triggers {
            result = match rule.rule_type {
                ReplacementRuleType::Simple => {
                    let replacement = rule.output.as_str();
                    if trigger.contains(' ') {
                        replace_phrase(&result, trigger, replacement)
                    } else {
                        replace_whole_word(&result, trigger, replacement)
                    }
                }
                ReplacementRuleType::Newline => replace_newline(&result, trigger),
                ReplacementRuleType::Wrap => {
                    wrap_next_word(&result, trigger, &rule.prefix, &rule.suffix, &rule.transform)
                }
            };
        }
    }
    result
}

fn replace_whole_word(text: &str, trigger: &str, replacement: &str) -> String {
    let pattern = format!(r"(?i)\b{}\b", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, replacement).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn replace_phrase(text: &str, trigger: &str, replacement: &str) -> String {
    let pattern = format!(r"(?i){}", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, replacement).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn replace_newline(text: &str, trigger: &str) -> String {
    let escaped = regex::escape(trigger);
    // Optional leading space absorbs the word-gap so no trailing whitespace is left
    // on the preceding line. Optional trailing punctuation is moved before the newline
    // so Whisper-added sentence endings (e.g. "new line?") don't land on the new line.
    let pattern = if trigger.contains(' ') {
        format!(r"(?i)[ ]?{}([.,!?;:]+)?[ ]*", escaped)
    } else {
        format!(r"(?i)[ ]?\b{}\b([.,!?;:]+)?[ ]*", escaped)
    };
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, |caps: &regex::Captures| {
            let punct = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            format!("{}\n", punct)
        }).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn wrap_next_word(text: &str, trigger: &str, prefix: &str, suffix: &str, transform: &WordTransform) -> String {
    // Match: whole-word trigger + whitespace + next non-whitespace word
    let pattern = format!(r"(?i)\b{}\s+(\S+)", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, |caps: &regex::Captures| {
            // Strip leading non-alphanumeric chars Whisper may have already inserted
            // (e.g. "hashtag #word" → Whisper pre-pended "#", avoid "##word")
            let raw = caps[1].trim_start_matches(|c: char| !c.is_alphanumeric());
            let word = apply_word_transform(if raw.is_empty() { &caps[1] } else { raw }, transform);
            format!("{}{}{}", prefix, word, suffix)
        }).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn apply_word_transform(word: &str, transform: &WordTransform) -> String {
    match transform {
        WordTransform::Lower => word.to_lowercase(),
        WordTransform::Upper => word.to_uppercase(),
        WordTransform::Sentence => {
            let lower = word.to_lowercase();
            let mut chars = lower.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
        WordTransform::None => word.to_string(),
    }
}

/// Returns "in", "out", or "" depending on the speaker label at the start of a segment.
/// Used by write_transcript to prevent merging across speaker sources.
fn speaker_source_prefix(text: &str) -> &'static str {
    if text.starts_with("in: ") { "in" }
    else if text.starts_with("out: ") { "out" }
    else { "" }
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

    fn wrap_rule(trigger: &str, prefix: &str, suffix: &str, transform: WordTransform) -> ReplacementRule {
        ReplacementRule {
            trigger: trigger.to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Wrap,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
            transform,
        }
    }

    fn newline_rule(trigger: &str) -> ReplacementRule {
        ReplacementRule {
            trigger: trigger.to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Newline,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        }
    }

    // ── cleanup_text ────────────────────────────────────────────────────────

    #[test]
    fn cleanup_splits_hashtag_newline_fusion() {
        // Whisper fuses "#cake" + "newline" into one token — must be split so rules fire
        assert_eq!(cleanup_text("#cakenewline"), "#cake newline");
    }


    #[test]
    fn cleanup_strips_silence_annotation() {
        assert_eq!(cleanup_text("[SILENCE] hello"), "hello");
    }

    #[test]
    fn cleanup_strips_blank_audio_annotation() {
        assert_eq!(cleanup_text("[BLANK_AUDIO]"), "");
    }

    #[test]
    fn cleanup_strips_general_bracket_annotation() {
        assert_eq!(cleanup_text("[MUSIC] welcome back [APPLAUSE]"), "welcome back");
    }

    #[test]
    fn cleanup_preserves_lowercase_brackets() {
        // User-facing [note] or [1] should not be stripped — only ALL-CAPS Whisper annotations
        assert_eq!(cleanup_text("see [note] below"), "see [note] below");
    }

    #[test]
    fn cleanup_normalizes_whitespace() {
        assert_eq!(cleanup_text("  hello   world  "), "hello world");
    }

    // ── dedup_consecutive_phrases ─────────────────────────────────────────────

    #[test]
    fn dedup_removes_consecutive_duplicate_words() {
        assert_eq!(dedup_consecutive_phrases("hello world world next"), "hello world next");
    }

    #[test]
    fn dedup_case_insensitive() {
        assert_eq!(dedup_consecutive_phrases("Hello hello world"), "Hello world");
    }

    #[test]
    fn dedup_does_not_remove_non_consecutive_duplicates() {
        assert_eq!(dedup_consecutive_phrases("hello world hello"), "hello world hello");
    }

    #[test]
    fn dedup_handles_punctuation_at_word_boundary() {
        // "world." and "world" are considered duplicates (strip trailing punct for compare)
        assert_eq!(dedup_consecutive_phrases("hello world. world next"), "hello world. next");
    }

    // ── apply_replacements ──────────────────────────────────────────────────

    #[test]
    fn replacements_simple_whole_word() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Both), "11 - may");
    }

    #[test]
    fn replacements_case_insensitive() {
        let rules = vec![simple_rule("hashtag", "#", ReplacementScope::Both)];
        assert_eq!(apply_replacements("HASHTAG project", &rules, &ReplacementScope::Both), "# project");
    }

    #[test]
    fn replacements_whole_word_not_substring() {
        let rules = vec![simple_rule("hash", "#", ReplacementScope::Both)];
        assert_eq!(apply_replacements("hashtag project", &rules, &ReplacementScope::Both), "hashtag project");
    }

    #[test]
    fn replacements_phrase_trigger() {
        let rules = vec![simple_rule("to do", "[ ]", ReplacementScope::Both)];
        assert_eq!(apply_replacements("add to do item", &rules, &ReplacementScope::Both), "add [ ] item");
    }

    #[test]
    fn replacements_newline_type() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(apply_replacements("hello new line world", &rules, &ReplacementScope::Both), "hello\nworld");
    }

    #[test]
    fn replace_newline_moves_trailing_question_mark() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements("looking for new line?", &rules, &ReplacementScope::Both),
            "looking for?\n"
        );
    }

    #[test]
    fn replace_newline_alias_single_word() {
        let rules = vec![ReplacementRule {
            trigger: "new line".to_string(),
            aliases: vec!["newline".to_string()],
            rule_type: ReplacementRuleType::Newline,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        }];
        assert_eq!(
            apply_replacements("go to bed newline", &rules, &ReplacementScope::Both),
            "go to bed\n"
        );
    }

    #[test]
    fn replacements_wrap_with_lower_transform() {
        let rules = vec![wrap_rule("hashtag", "#", "", WordTransform::Lower)];
        assert_eq!(apply_replacements("hashtag Monday", &rules, &ReplacementScope::Both), "#monday");
    }

    #[test]
    fn replacements_wrap_leaves_rest_unchanged() {
        let rules = vec![wrap_rule("bold", "**", "**", WordTransform::None)];
        assert_eq!(apply_replacements("bold hello world", &rules, &ReplacementScope::Both), "**hello** world");
    }

    #[test]
    fn replacements_scope_transcripts_skips_dictate_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Dictate)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Transcripts), "11 dash may");
    }

    #[test]
    fn replacements_scope_dictate_skips_transcripts_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Transcripts)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Dictate), "11 dash may");
    }

    #[test]
    fn replacements_both_scope_applies_to_transcripts() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("a dash b", &rules, &ReplacementScope::Transcripts), "a - b");
    }

    #[test]
    fn replacements_multiple_rules_in_order() {
        let rules = vec![
            simple_rule("hashtag", "#", ReplacementScope::Both),
            simple_rule("todo", "[ ]", ReplacementScope::Both),
        ];
        assert_eq!(
            apply_replacements("hashtag project todo item", &rules, &ReplacementScope::Both),
            "# project [ ] item"
        );
    }

    #[test]
    fn replacements_empty_trigger_skipped() {
        let rules = vec![simple_rule("", "oops", ReplacementScope::Both)];
        assert_eq!(apply_replacements("hello", &rules, &ReplacementScope::Both), "hello");
    }


    fn temp_file(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liscribe-output-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join(name)
    }

    #[test]
    fn transcript_renders_timestamps_when_enabled() {
        let svc = OutputService;
        let file = temp_file("with-timestamps.md");
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
        }];

        svc.write_transcript(&segments, &[], "Test", "tiny", true, &[], &file)
            .expect("write transcript");

        let content = std::fs::read_to_string(&file).expect("read transcript");
        assert!(content.contains("[00:00:12] hello world"));
    }

    #[test]
    fn transcript_omits_timestamps_when_disabled() {
        let svc = OutputService;
        let file = temp_file("without-timestamps.md");
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
        }];

        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], &file)
            .expect("write transcript");

        let content = std::fs::read_to_string(&file).expect("read transcript");
        assert!(content.contains("hello world"));
        assert!(!content.contains("[00:00:12]"));
    }

    #[test]
    fn dual_source_segments_are_never_merged_across_speaker_boundary() {
        // "in: yeah" ends at 1000 ms; "out: Hello." starts at 1200 ms — gap is only 200 ms,
        // but they must NOT merge because the source labels differ.
        let svc = OutputService;
        let file = temp_file("dual-source-newlines.md");
        let segments = vec![
            Segment { start_ms: 0, end_ms: 1_000, text: "in: yeah".to_string() },
            Segment { start_ms: 1_200, end_ms: 3_000, text: "out: Hello there.".to_string() },
            Segment { start_ms: 3_100, end_ms: 4_000, text: "out: How are you?".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        // Speaker change uses a blank line
        assert!(
            content.contains("in: yeah\n\nout: Hello there."),
            "in: and out: should be separated by a blank line, got:\n{content}"
        );
        // Two consecutive "out:" segments within gap should merge without repeating the label
        assert!(
            content.contains("out: Hello there. How are you?"),
            "consecutive out: segments within gap should merge without repeating label, got:\n{content}"
        );
    }

    #[test]
    fn dual_source_speaker_change_uses_blank_line() {
        let svc = OutputService;
        let file = temp_file("dual-source-compact.md");
        let segments = vec![
            Segment { start_ms: 0, end_ms: 1_000, text: "in: yeah".to_string() },
            Segment { start_ms: 2_000, end_ms: 4_000, text: "out: Thanks for sharing.".to_string() },
            Segment { start_ms: 5_000, end_ms: 6_000, text: "in: Absolutely.".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        // Each speaker change: blank line \n\n
        assert!(content.contains("in: yeah\n\nout:"), "in→out should be \\n\\n, got:\n{content}");
        assert!(content.contains("out: Thanks for sharing.\n\nin:"), "out→in should be \\n\\n, got:\n{content}");
    }

    #[test]
    fn single_source_always_uses_double_newline() {
        let svc = OutputService;
        let file = temp_file("single-source-separator.md");
        // Two segments with a gap > 8 s so they stay separate paragraphs
        let segments = vec![
            Segment { start_ms: 0, end_ms: 2_000, text: "First thought.".to_string() },
            Segment { start_ms: 12_000, end_ms: 14_000, text: "Second thought.".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        assert!(
            content.contains("First thought.\n\nSecond thought."),
            "single-source paragraphs should use \\n\\n, got:\n{content}"
        );
    }

    #[test]
    fn single_source_segments_still_merge_within_gap() {
        let svc = OutputService;
        let file = temp_file("single-source-merge.md");
        let segments = vec![
            Segment { start_ms: 0, end_ms: 500, text: "Hello".to_string() },
            Segment { start_ms: 700, end_ms: 1_200, text: "world.".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        assert!(
            content.contains("Hello world."),
            "same-source segments within gap should merge, got:\n{content}"
        );
    }

    #[test]
    fn session_notes_json_includes_title_wav_and_note_text() {
        let svc = OutputService;
        let dir = std::env::temp_dir().join(format!("liscribe-notes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let notes = vec![Note {
            id: "n1".to_string(),
            text: "remember this".to_string(),
            recorded_at_ms: 2500,
        }];
        let dest = svc
            .write_session_notes(&dir, "Meeting A", "mic.wav", &notes)
            .expect("write_session_notes");
        assert_eq!(dest.file_name().unwrap(), "notes.json");
        let raw = std::fs::read_to_string(&dest).expect("read");
        assert!(raw.contains("Meeting A"));
        assert!(raw.contains("mic.wav"));
        assert!(raw.contains("remember this"));
    }

    // ── format_ms ────────────────────────────────────────────────────────────

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

    // ── transcript_path slug ─────────────────────────────────────────────────

    #[test]
    fn transcript_path_replaces_spaces_with_underscores() {
        let svc = OutputService;
        let dir = std::env::temp_dir();
        let model = std::path::Path::new("/models/ggml-tiny.bin");
        let path = svc.transcript_path(&dir, model, "my title");
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("my_title_"));
    }

    #[test]
    fn transcript_path_replaces_forbidden_chars_with_dashes() {
        let svc = OutputService;
        let dir = std::env::temp_dir();
        let model = std::path::Path::new("/models/ggml-tiny.bin");
        let path = svc.transcript_path(&dir, model, "foo/bar:baz");
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("foo-bar-baz_"));
    }

    // ── write_dictate_history_entry / read_dictate_history ───────────────────

    fn temp_save_folder() -> String {
        let dir = std::env::temp_dir()
            .join(format!("liscribe-dictate-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn read_dictate_history_returns_empty_when_file_missing() {
        let svc = OutputService;
        let folder = temp_save_folder();
        let entries = svc.read_dictate_history(&folder).expect("read");
        assert!(entries.is_empty());
    }

    #[test]
    fn write_dictate_history_creates_missing_save_folder() {
        let svc = OutputService;
        let folder = std::env::temp_dir()
            .join(format!(
                "liscribe-dictate-mkdir-tests-{}",
                uuid::Uuid::new_v4()
            ))
            .join("nested")
            .join("save");
        let folder = folder.to_string_lossy().to_string();
        assert!(!PathBuf::from(&folder).exists());

        svc.write_dictate_history_entry(&folder, "hello")
            .expect("write creates parent dirs");

        let entries = svc.read_dictate_history(&folder).expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "hello");
    }

    #[test]
    fn write_then_read_dictate_history_entry() {
        let svc = OutputService;
        let folder = temp_save_folder();

        svc.write_dictate_history_entry(&folder, "hello world").expect("write");

        let entries = svc.read_dictate_history(&folder).expect("read");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "hello world");
    }

    #[test]
    fn dictate_history_entries_are_newest_first() {
        let svc = OutputService;
        let folder = temp_save_folder();

        svc.write_dictate_history_entry(&folder, "first").expect("write first");
        svc.write_dictate_history_entry(&folder, "second").expect("write second");

        let entries = svc.read_dictate_history(&folder).expect("read");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second");
        assert_eq!(entries[1].text, "first");
    }

    #[test]
    fn dictate_history_entry_has_non_empty_id_and_timestamp() {
        let svc = OutputService;
        let folder = temp_save_folder();

        svc.write_dictate_history_entry(&folder, "test entry").expect("write");

        let entries = svc.read_dictate_history(&folder).expect("read");
        assert!(!entries[0].id.is_empty());
        assert!(!entries[0].timestamp.is_empty());
    }
}
