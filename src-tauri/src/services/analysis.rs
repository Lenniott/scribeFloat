//! Shared RMS helper.
//!
//! Pure module: no I/O, no locks. `rms()` is the crate's canonical RMS
//! helper — hallucination gating (`services/output/hallucination.rs`)
//! delegates to it for speaker-silence detection.
//!
//! This module used to host live pitch/loudness voice-change-cut detection
//! (ADR-0013). That machinery was superseded by Sortformer diarization
//! (ADR-0014) and had zero consumers, so it was removed; see
//! `.scratch/context-chunking-strategy/issues/01-remove-dead-pitch-loudness-analyzer.md`.

/// Root-mean-square level. Canonical RMS for the crate — other modules
/// (hallucination gating, level metering) should delegate here.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_matches_known_signal() {
        // Full-scale sine has RMS 1/sqrt(2).
        let n = 16_000;
        let wave: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 100.0 * i as f32 / 16_000.0).sin())
            .collect();
        assert!((rms(&wave) - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01);
        assert_eq!(rms(&[]), 0.0);
    }
}
