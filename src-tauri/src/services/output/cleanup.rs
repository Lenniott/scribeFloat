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
/// Always-on — these are never valid speech output.
pub(crate) fn cleanup_text(text: &str) -> String {
    // Strip uppercase-first Whisper bracket annotations: [BLANK_AUDIO], [Music], [Applause], etc.
    // Requires first char uppercase so user annotations like [note] are preserved.
    let cleaned = CAPS_RE.replace_all(text, "");
    // Also strip known lowercase Whisper noise tokens emitted by the VAD path.
    let cleaned = NOISE_RE.replace_all(&cleaned, "");
    // Whisper sometimes fuses "#word" with a following command word
    // (e.g. "hashtag cake new line" → "#cakenewline"). Split so replacement rules fire.
    let cleaned = FUSION_RE.replace_all(&cleaned, "$1 $2");
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_hashtag_newline_fusion() {
        assert_eq!(cleanup_text("#cakenewline"), "#cake newline");
    }

    #[test]
    fn strips_silence_annotation() {
        assert_eq!(cleanup_text("[SILENCE] hello"), "hello");
    }

    #[test]
    fn strips_blank_audio_annotation() {
        assert_eq!(cleanup_text("[BLANK_AUDIO]"), "");
    }

    #[test]
    fn strips_uppercase_bracket_annotations() {
        assert_eq!(
            cleanup_text("[MUSIC] welcome back [APPLAUSE]"),
            "welcome back"
        );
    }

    #[test]
    fn preserves_lowercase_user_annotations() {
        // [note] and [1] are user-facing; only ALL-CAPS Whisper annotations are stripped.
        assert_eq!(cleanup_text("see [note] below"), "see [note] below");
    }

    #[test]
    fn strips_lowercase_whisper_noise_tokens() {
        // VAD path emits these in lowercase on some Whisper builds.
        assert_eq!(cleanup_text("[silence] hello"), "hello");
        assert_eq!(cleanup_text("[blank_audio]"), "");
        assert_eq!(cleanup_text("[music] intro [applause]"), "intro");
        assert_eq!(cleanup_text("[inaudible] world"), "world");
    }

    #[test]
    fn normalizes_internal_whitespace() {
        assert_eq!(cleanup_text("  hello   world  "), "hello world");
    }
}
