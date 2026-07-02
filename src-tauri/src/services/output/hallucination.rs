use crate::types::Segment;

/// RMS below −60 dBFS — speaker channel treated as silent (no transcription).
pub const SPEAKER_SILENCE_THRESHOLD: f32 = 1e-3;

/// Root-mean-square amplitude of mono PCM samples.
pub fn pcm_rms(pcm: &[f32]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = pcm.iter().map(|&s| s * s).sum();
    (sum_sq / pcm.len() as f32).sqrt()
}

/// Returns true when assembled speaker PCM is loud enough to transcribe.
pub fn speaker_pcm_has_signal(pcm: &[f32]) -> bool {
    pcm_rms(pcm) >= SPEAKER_SILENCE_THRESHOLD
}

/// Strip known Whisper hallucination phrases from segments.
/// Whisper frequently outputs these on silent or near-silent input.
pub fn filter_hallucination_phrases(segments: &[Segment]) -> Vec<Segment> {
    const PHRASES: &[&str] = &[
        "thank you.",
        "thanks.",
        "thanks for watching.",
        "thank you for watching.",
        "thank you for listening.",
        "please subscribe.",
        "see you next time.",
        "you.",
        "bye.",
        "bye-bye.",
        "bye bye.",
    ];
    segments
        .iter()
        .filter(|seg| {
            let lower = seg.text.trim().to_lowercase();
            if lower.is_empty() {
                return false;
            }
            if PHRASES.iter().any(|&p| lower == p) {
                return false;
            }
            if lower.starts_with("transcribed by") || lower.starts_with("subtitles by") {
                return false;
            }
            if is_bracket_only_hallucination(&lower) {
                return false;
            }
            true
        })
        .cloned()
        .collect()
}

fn is_bracket_only_hallucination(lower: &str) -> bool {
    let trimmed = lower.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return false;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    matches!(
        inner,
        "music"
            | "applause"
            | "laughter"
            | "silence"
            | "blank_audio"
            | "no_speech"
            | "noise"
            | "inaudible"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Segment;

    fn seg(text: &str) -> Segment {
        Segment {
            start_ms: 0,
            end_ms: 500,
            text: text.to_string(),
            source: None,
        }
    }

    // ── pcm_rms ───────────────────────────────────────────────────────────────

    #[test]
    fn pcm_rms_empty_returns_zero() {
        assert_eq!(pcm_rms(&[]), 0.0);
    }

    #[test]
    fn pcm_rms_silence_returns_zero() {
        assert_eq!(pcm_rms(&vec![0.0f32; 1000]), 0.0);
    }

    #[test]
    fn pcm_rms_dc_full_scale_returns_one() {
        let rms = pcm_rms(&vec![1.0f32; 1000]);
        assert!((rms - 1.0).abs() < 1e-5);
    }

    #[test]
    fn pcm_rms_below_threshold_for_silence() {
        assert!(!speaker_pcm_has_signal(&vec![0.0f32; 16_000]));
    }

    #[test]
    fn pcm_rms_above_threshold_for_real_audio() {
        assert!(speaker_pcm_has_signal(&vec![0.05f32; 16_000]));
    }

    // ── filter_hallucination_phrases ──────────────────────────────────────────

    #[test]
    fn filter_removes_known_hallucination_phrases() {
        let segs = vec![
            seg("Thank you."),
            seg("Hello world"),
            seg("Thanks for watching."),
            seg("Transcribed by Whisper"),
        ];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Hello world");
    }

    #[test]
    fn filter_is_case_insensitive() {
        let segs = vec![seg("THANK YOU."), seg("Real speech here")];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Real speech here");
    }

    #[test]
    fn filter_keeps_real_speech_intact() {
        let segs = vec![
            seg("I'm talking about the project"),
            seg("Let me explain the architecture"),
        ];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_does_not_strip_thank_you_mid_sentence() {
        let segs = vec![seg("I want to thank you for coming today")];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn filter_removes_youtube_outro_phrases() {
        let segs = vec![
            seg("Thank you for listening."),
            seg("Please subscribe."),
            seg("See you next time."),
            seg("Actual content."),
        ];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Actual content.");
    }

    #[test]
    fn filter_removes_subtitles_by_prefix() {
        let segs = vec![seg("Subtitles by the Amara.org community")];
        let filtered = filter_hallucination_phrases(&segs);
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_removes_bracket_only_music() {
        let segs = vec![seg("[Music]"), seg("Real speech.")];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Real speech.");
    }

    #[test]
    fn filter_keeps_bracket_annotations_mid_sentence() {
        let segs = vec![seg("Welcome back [music] everyone")];
        let filtered = filter_hallucination_phrases(&segs);
        assert_eq!(filtered.len(), 1);
    }
}
