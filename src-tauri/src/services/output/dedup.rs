/// Remove consecutively repeated phrases of 1–5 words (case-insensitive).
/// Handles Whisper repetition artifacts at segment boundaries:
///   "hello world. world. Next"  → "hello world. Next"
///   "eat some food eat some food" → "eat some food"
pub(crate) fn dedup_consecutive_phrases(text: &str) -> String {
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

/// When Whisper (or a double-paste bug) yields the same paragraph twice back-to-back, keep one copy.
pub(super) fn dedup_exact_halves(text: &str) -> String {
    let trimmed = text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 {
        return text.to_string();
    }
    // Use a character-count midpoint so split_at always lands on a char boundary,
    // even when `trimmed` contains multi-byte UTF-8 characters (accents, emoji, etc.).
    let mid_byte = trimmed
        .char_indices()
        .nth(char_count / 2)
        .map(|(i, _)| i)
        .unwrap_or(trimmed.len());
    let first = &trimmed[..mid_byte];
    let second = &trimmed[mid_byte..];
    if first.trim() == second.trim() {
        first.trim().to_string()
    } else {
        text.to_string()
    }
}

/// If the transcript appears twice (Whisper hallucination on long audio), keep the first copy.
/// Uses the opening fingerprint (~20% of text, ≤100 chars) to detect the repeat start.
pub(crate) fn dedup_repeated_block(text: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_exact_halves_removes_verbatim_repeat() {
        let once = "No, I'm using dictate right now to test.";
        let twice = format!("{once}{once}");
        assert_eq!(dedup_exact_halves(&twice), once);
    }

    #[test]
    fn dedup_removes_consecutive_duplicate_words() {
        assert_eq!(
            dedup_consecutive_phrases("hello world world next"),
            "hello world next"
        );
    }

    #[test]
    fn dedup_case_insensitive() {
        assert_eq!(
            dedup_consecutive_phrases("Hello hello world"),
            "Hello world"
        );
    }

    #[test]
    fn dedup_does_not_remove_non_consecutive_duplicates() {
        assert_eq!(
            dedup_consecutive_phrases("hello world hello"),
            "hello world hello"
        );
    }

    #[test]
    fn dedup_handles_punctuation_at_word_boundary() {
        assert_eq!(
            dedup_consecutive_phrases("hello world. world next"),
            "hello world. next"
        );
    }

    #[test]
    fn dedup_exact_halves_empty() {
        assert_eq!(dedup_exact_halves(""), "");
    }

    #[test]
    fn dedup_exact_halves_ascii_dedup() {
        assert_eq!(dedup_exact_halves("hello hello"), "hello");
    }

    #[test]
    fn dedup_exact_halves_no_dedup() {
        assert_eq!(dedup_exact_halves("hello world"), "hello world");
    }

    #[test]
    fn dedup_exact_halves_non_ascii_no_panic() {
        let result = dedup_exact_halves("café café");
        assert_eq!(result, "café");
    }
}
