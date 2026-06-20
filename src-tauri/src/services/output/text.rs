use crate::types::{ReplacementRule, ReplacementRuleType, ReplacementScope, WordTransform};
use regex::Regex;
use std::sync::LazyLock;

pub(crate) static CAPS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[[A-Z][A-Za-z_ ]*\]").expect("static regex")
});

pub(crate) static NOISE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\[(silence|blank_audio|no_speech|music|applause|laughter|noise|inaudible)\]",
    )
    .expect("static regex")
});

pub(crate) static FUSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(#\w+?)(newline)").expect("static regex")
});

/// Strip Whisper artifact annotations and normalize whitespace from a single segment.
pub(crate) fn cleanup_text(text: &str) -> String {
    let cleaned = CAPS_RE.replace_all(text, "");
    let cleaned = NOISE_RE.replace_all(&cleaned, "");
    let cleaned = FUSION_RE.replace_all(&cleaned, "$1 $2");
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    words.join(" ")
}

/// Apply user-defined replacement rules to text. Rules are applied in order.
pub(crate) fn apply_replacements(
    text: &str,
    rules: &[ReplacementRule],
    scope: &ReplacementScope,
    prefix: &str,
) -> String {
    let mut result = text.to_string();
    for rule in rules {
        if rule.trigger.trim().is_empty() {
            continue;
        }
        let rule_scope = &rule.scope;
        if rule_scope != &ReplacementScope::Both && rule_scope != scope {
            continue;
        }
        let triggers: Vec<String> = std::iter::once(rule.trigger.as_str())
            .chain(rule.aliases.iter().map(String::as_str))
            .filter(|t| !t.trim().is_empty())
            .map(|t| effective_trigger(t, prefix))
            .collect();
        for trigger in &triggers {
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
                ReplacementRuleType::Wrap => wrap_next_word(
                    &result,
                    trigger,
                    &rule.prefix,
                    &rule.suffix,
                    &rule.transform,
                ),
            };
        }
    }
    result
}

fn effective_trigger(trigger: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return trigger.to_string();
    }
    let prefix_space = format!("{} ", prefix);
    if trigger.starts_with(&prefix_space) {
        trigger.to_string()
    } else {
        format!("{} {}", prefix, trigger)
    }
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
    let pattern = if trigger.contains(' ') {
        format!(r"(?i)[ ]?{}([.,!?;:]+)?[ ]*", escaped)
    } else {
        format!(r"(?i)[ ]?\b{}\b([.,!?;:]+)?[ ]*", escaped)
    };
    match Regex::new(&pattern) {
        Ok(re) => re
            .replace_all(text, |caps: &regex::Captures| {
                let punct = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                format!("{}\n", punct)
            })
            .into_owned(),
        Err(_) => text.to_string(),
    }
}

fn wrap_next_word(
    text: &str,
    trigger: &str,
    prefix: &str,
    suffix: &str,
    transform: &WordTransform,
) -> String {
    let pattern = format!(r"(?i)\b{}\s+(\S+)", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re
            .replace_all(text, |caps: &regex::Captures| {
                let raw = caps[1].trim_start_matches(|c: char| !c.is_alphanumeric());
                let word =
                    apply_word_transform(if raw.is_empty() { &caps[1] } else { raw }, transform);
                format!("{}{}{}", prefix, word, suffix)
            })
            .into_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ReplacementRule, ReplacementRuleType, WordTransform};

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

    fn wrap_rule(
        trigger: &str,
        prefix: &str,
        suffix: &str,
        transform: WordTransform,
    ) -> ReplacementRule {
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

    #[test]
    fn cleanup_splits_hashtag_newline_fusion() {
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
        assert_eq!(
            cleanup_text("[MUSIC] welcome back [APPLAUSE]"),
            "welcome back"
        );
    }

    #[test]
    fn cleanup_preserves_lowercase_brackets() {
        assert_eq!(cleanup_text("see [note] below"), "see [note] below");
    }

    #[test]
    fn cleanup_strips_lowercase_whisper_noise_tokens() {
        assert_eq!(cleanup_text("[silence] hello"), "hello");
        assert_eq!(cleanup_text("[blank_audio]"), "");
        assert_eq!(cleanup_text("[music] intro [applause]"), "intro");
        assert_eq!(cleanup_text("[inaudible] world"), "world");
    }

    #[test]
    fn cleanup_normalizes_whitespace() {
        assert_eq!(cleanup_text("  hello   world  "), "hello world");
    }

    #[test]
    fn replacements_simple_whole_word() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("11 dash may", &rules, &ReplacementScope::Both, ""),
            "11 - may"
        );
    }

    #[test]
    fn replacements_case_insensitive() {
        let rules = vec![simple_rule("hashtag", "#", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("HASHTAG project", &rules, &ReplacementScope::Both, ""),
            "# project"
        );
    }

    #[test]
    fn replacements_whole_word_not_substring() {
        let rules = vec![simple_rule("hash", "#", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("hashtag project", &rules, &ReplacementScope::Both, ""),
            "hashtag project"
        );
    }

    #[test]
    fn replacements_phrase_trigger() {
        let rules = vec![simple_rule("to do", "[ ]", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("add to do item", &rules, &ReplacementScope::Both, ""),
            "add [ ] item"
        );
    }

    #[test]
    fn replacements_newline_type() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements("hello new line world", &rules, &ReplacementScope::Both, ""),
            "hello\nworld"
        );
    }

    #[test]
    fn replace_newline_moves_trailing_question_mark() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements("looking for new line?", &rules, &ReplacementScope::Both, ""),
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
            apply_replacements("go to bed newline", &rules, &ReplacementScope::Both, ""),
            "go to bed\n"
        );
    }

    #[test]
    fn replacements_wrap_with_lower_transform() {
        let rules = vec![wrap_rule("hashtag", "#", "", WordTransform::Lower)];
        assert_eq!(
            apply_replacements("hashtag Monday", &rules, &ReplacementScope::Both, ""),
            "#monday"
        );
    }

    #[test]
    fn replacements_wrap_leaves_rest_unchanged() {
        let rules = vec![wrap_rule("bold", "**", "**", WordTransform::None)];
        assert_eq!(
            apply_replacements("bold hello world", &rules, &ReplacementScope::Both, ""),
            "**hello** world"
        );
    }

    #[test]
    fn replacements_scope_transcripts_skips_dictate_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Dictate)];
        assert_eq!(
            apply_replacements("11 dash may", &rules, &ReplacementScope::Transcripts, ""),
            "11 dash may"
        );
    }

    #[test]
    fn replacements_scope_dictate_skips_transcripts_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Transcripts)];
        assert_eq!(
            apply_replacements("11 dash may", &rules, &ReplacementScope::Dictate, ""),
            "11 dash may"
        );
    }

    #[test]
    fn replacements_both_scope_applies_to_transcripts() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("a dash b", &rules, &ReplacementScope::Transcripts, ""),
            "a - b"
        );
    }

    #[test]
    fn replacements_multiple_rules_in_order() {
        let rules = vec![
            simple_rule("hashtag", "#", ReplacementScope::Both),
            simple_rule("todo", "[ ]", ReplacementScope::Both),
        ];
        assert_eq!(
            apply_replacements(
                "hashtag project todo item",
                &rules,
                &ReplacementScope::Both,
                ""
            ),
            "# project [ ] item"
        );
    }

    #[test]
    fn replacements_empty_trigger_skipped() {
        let rules = vec![simple_rule("", "oops", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("hello", &rules, &ReplacementScope::Both, ""),
            "hello"
        );
    }

    #[test]
    fn old_format_rule_with_empty_prefix_matches() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("11 float dash may", &rules, &ReplacementScope::Both, ""),
            "11 - may"
        );
    }

    #[test]
    fn old_format_bare_word_does_not_match() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("11 dash may", &rules, &ReplacementScope::Both, ""),
            "11 dash may"
        );
    }

    #[test]
    fn old_format_newline_rule_with_empty_prefix_matches() {
        let rules = vec![newline_rule("float new line")];
        assert_eq!(
            apply_replacements(
                "hello float new line world",
                &rules,
                &ReplacementScope::Both,
                ""
            ),
            "hello\nworld"
        );
    }

    #[test]
    fn old_format_bare_new_line_does_not_match() {
        let rules = vec![newline_rule("float new line")];
        assert_eq!(
            apply_replacements("hello new line world", &rules, &ReplacementScope::Both, ""),
            "hello new line world"
        );
    }

    #[test]
    fn prefix_prepended_to_base_trigger_fires() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements(
                "eleven float dash may",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "eleven - may"
        );
    }

    #[test]
    fn prefix_base_trigger_alone_does_not_fire() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("eleven dash may", &rules, &ReplacementScope::Both, "float"),
            "eleven dash may"
        );
    }

    #[test]
    fn prefix_not_double_applied_to_old_format_rule() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements(
                "eleven float dash may",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "eleven - may"
        );
    }

    #[test]
    fn prefix_double_prefixed_text_matches_embedded_phrase() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements(
                "eleven float float dash may",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "eleven float - may"
        );
    }

    #[test]
    fn empty_prefix_fires_base_trigger_directly() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("eleven dash may", &rules, &ReplacementScope::Both, ""),
            "eleven - may"
        );
    }

    #[test]
    fn prefix_newline_rule_base_trigger() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements(
                "hello float new line world",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "hello\nworld"
        );
    }

    #[test]
    fn prefix_newline_bare_trigger_does_not_fire() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements(
                "hello new line world",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "hello new line world"
        );
    }
}
