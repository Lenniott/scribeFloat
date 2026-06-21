use crate::types::{ReplacementRule, ReplacementRuleType, ReplacementScope, WordTransform};
use regex::Regex;

/// Apply user-defined replacement rules to text. Rules are applied in order.
/// Only rules whose scope is Both or matches the given scope are applied.
/// `prefix` is the global command prefix (default "float") — it is prepended to
/// each trigger at match time so triggers are stored without it in the Config.
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
                    replace_whole_word(&result, trigger, rule.output.as_str())
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

/// Returns the effective trigger string, prepending `prefix` when the trigger doesn't
/// already include it. This allows old-format rules ("float dash") and new-format rules
/// ("dash" with prefix="float") to coexist and produce identical behaviour.
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
    // Match: whole-word trigger + whitespace + next non-whitespace word
    let pattern = format!(r"(?i)\b{}\s+(\S+)", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re
            .replace_all(text, |caps: &regex::Captures| {
                // Strip leading non-alphanumeric chars Whisper may have already inserted
                // (e.g. "hashtag #word" → Whisper pre-pended "#", avoid "##word")
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

    // All tests use prefix="float" — the app default — to reflect real usage.
    // The "float" prefix is prepended to each trigger at match time; triggers are
    // stored WITHOUT it in Config::replacement_rules.

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

    // ── Rule types — single-word trigger ─────────────────────────────────────

    #[test]
    fn whole_word_rule_fires_with_float_prefix() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("11 float dash may", &rules, &ReplacementScope::Both, "float"),
            "11 - may"
        );
    }

    #[test]
    fn whole_word_rule_case_insensitive() {
        let rules = vec![simple_rule("hashtag", "#", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("float HASHTAG project", &rules, &ReplacementScope::Both, "float"),
            "# project"
        );
    }

    #[test]
    fn bare_trigger_does_not_fire_when_prefix_set() {
        // "hashtag project" without "float" prefix word → no substitution.
        let rules = vec![simple_rule("hashtag", "#", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("hashtag project", &rules, &ReplacementScope::Both, "float"),
            "hashtag project"
        );
    }

    #[test]
    fn whole_word_does_not_match_substring() {
        // trigger "hash" should not fire inside "hashtag"
        let rules = vec![simple_rule("hash", "#", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("float hashtag project", &rules, &ReplacementScope::Both, "float"),
            "float hashtag project"
        );
    }

    // ── Rule types — phrase trigger ───────────────────────────────────────────

    #[test]
    fn phrase_rule_fires_with_float_prefix() {
        let rules = vec![simple_rule("to do", "[ ]", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("add float to do item", &rules, &ReplacementScope::Both, "float"),
            "add [ ] item"
        );
    }

    // ── Rule types — newline ──────────────────────────────────────────────────

    #[test]
    fn newline_rule_fires_with_float_prefix() {
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
    fn newline_rule_bare_trigger_does_not_fire_with_prefix() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements("hello new line world", &rules, &ReplacementScope::Both, "float"),
            "hello new line world"
        );
    }

    #[test]
    fn newline_rule_moves_trailing_punctuation_before_break() {
        // "float new line?" → the "?" goes to the preceding line, not the new one
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements(
                "looking for float new line?",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "looking for?\n"
        );
    }

    #[test]
    fn newline_rule_alias_fires_with_float_prefix() {
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
            apply_replacements("go to bed float newline", &rules, &ReplacementScope::Both, "float"),
            "go to bed\n"
        );
    }

    // ── Rule types — wrap ─────────────────────────────────────────────────────

    #[test]
    fn wrap_rule_applies_transform_and_prefix_suffix() {
        let rules = vec![wrap_rule("hashtag", "#", "", WordTransform::Lower)];
        assert_eq!(
            apply_replacements("float hashtag Monday", &rules, &ReplacementScope::Both, "float"),
            "#monday"
        );
    }

    #[test]
    fn wrap_rule_leaves_surrounding_text_unchanged() {
        let rules = vec![wrap_rule("bold", "**", "**", WordTransform::None)];
        assert_eq!(
            apply_replacements(
                "float bold hello world",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "**hello** world"
        );
    }

    // ── Scope filtering ───────────────────────────────────────────────────────

    #[test]
    fn transcripts_scope_skips_dictate_only_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Dictate)];
        assert_eq!(
            apply_replacements(
                "11 float dash may",
                &rules,
                &ReplacementScope::Transcripts,
                "float"
            ),
            "11 float dash may"
        );
    }

    #[test]
    fn dictate_scope_skips_transcripts_only_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Transcripts)];
        assert_eq!(
            apply_replacements("11 float dash may", &rules, &ReplacementScope::Dictate, "float"),
            "11 float dash may"
        );
    }

    #[test]
    fn both_scope_rule_applies_in_transcripts_context() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("a float dash b", &rules, &ReplacementScope::Transcripts, "float"),
            "a - b"
        );
    }

    // ── Multiple rules ────────────────────────────────────────────────────────

    #[test]
    fn multiple_rules_applied_in_order() {
        let rules = vec![
            simple_rule("hashtag", "#", ReplacementScope::Both),
            simple_rule("todo", "[ ]", ReplacementScope::Both),
        ];
        assert_eq!(
            apply_replacements(
                "float hashtag project float todo item",
                &rules,
                &ReplacementScope::Both,
                "float"
            ),
            "# project [ ] item"
        );
    }

    #[test]
    fn empty_trigger_is_skipped() {
        let rules = vec![simple_rule("", "oops", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("hello", &rules, &ReplacementScope::Both, "float"),
            "hello"
        );
    }

    // ── Old-format backward compat (prefix embedded in trigger string) ────────
    // Old rules stored the full "float dash" trigger. New rules store just "dash"
    // and rely on the engine prefix. Both formats must coexist.

    #[test]
    fn old_format_embedded_prefix_fires() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("11 float dash may", &rules, &ReplacementScope::Both, "float"),
            "11 - may"
        );
    }

    #[test]
    fn old_format_bare_word_does_not_match() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("11 dash may", &rules, &ReplacementScope::Both, "float"),
            "11 dash may"
        );
    }

    #[test]
    fn old_format_prefix_not_doubled() {
        // Old-format trigger already has "float "; engine must not produce "float float dash".
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("eleven float dash may", &rules, &ReplacementScope::Both, "float"),
            "eleven - may"
        );
    }

    #[test]
    fn old_format_newline_rule_fires() {
        let rules = vec![newline_rule("float new line")];
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
    fn old_format_newline_bare_trigger_does_not_match() {
        let rules = vec![newline_rule("float new line")];
        assert_eq!(
            apply_replacements("hello new line world", &rules, &ReplacementScope::Both, "float"),
            "hello new line world"
        );
    }

    // ── Empty prefix (no command word configured) ─────────────────────────────

    #[test]
    fn empty_prefix_fires_bare_trigger_directly() {
        // When the user clears their prefix, triggers fire on the bare word.
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(
            apply_replacements("eleven dash may", &rules, &ReplacementScope::Both, ""),
            "eleven - may"
        );
    }
}
