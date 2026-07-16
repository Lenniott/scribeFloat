//! Sortformer v2 speaker diarization: anonymous "who spoke when" spans.
//!
//! Owns the ONNX model lifetime; controllers never touch inference directly.
//! Two capabilities behind small traits so the transcription seam and the live
//! worker loop are testable without model files:
//! - [`Diarizer`]: one full-audio pass (Upload).
//! - [`StreamingDiarizer`]: incremental feed/flush (live Record worker, added
//!   with the capture wiring).
//!
//! No identity, no embeddings: output is [`DiarizationRange`] slots 0..=3.

use crate::types::DiarizationRange;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const SORTFORMER_MODEL_FILENAME: &str = "diar_streaming_sortformer_4spk-v2.onnx";

/// Sortformer emits sample offsets at 16 kHz; the app timeline is ms.
fn samples_to_ms(samples: u64) -> u64 {
    samples * 1_000 / 16_000
}

/// Map one Sortformer span (16 kHz sample offsets) into the app's ms timeline.
fn range_from_samples(speaker_id: usize, start_samples: u64, end_samples: u64) -> DiarizationRange {
    DiarizationRange {
        speaker_id: speaker_id.min(u8::MAX as usize) as u8,
        start_ms: samples_to_ms(start_samples),
        end_ms: samples_to_ms(end_samples),
    }
}

/// Full-audio diarization capability (Upload path). `Err` degrades to a plain
/// transcript at the seam; it never fails the note.
pub trait Diarizer: Sync {
    fn diarize(&self, pcm_16k: &[f32]) -> Result<Vec<DiarizationRange>>;
}

pub struct DiarizationService {
    model_path: PathBuf,
}

impl DiarizationService {
    pub fn new(model_path: PathBuf) -> Arc<Self> {
        Arc::new(Self { model_path })
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Present and non-empty. Dev builds ship 0-byte placeholder resources, so
    /// zero-length files count as missing.
    pub fn model_available(&self) -> bool {
        std::fs::metadata(&self.model_path)
            .map(|m| m.is_file() && m.len() > 0)
            .unwrap_or(false)
    }

    fn load_sortformer(&self) -> Result<parakeet_rs::sortformer::Sortformer> {
        anyhow::ensure!(
            self.model_available(),
            "diarization model missing: {}",
            self.model_path.display()
        );
        parakeet_rs::sortformer::Sortformer::with_config(
            &self.model_path,
            None,
            parakeet_rs::sortformer::DiarizationConfig::callhome(),
        )
        .map_err(|e| anyhow::anyhow!("failed to load diarization model: {e}"))
    }
}

impl Diarizer for DiarizationService {
    fn diarize(&self, pcm_16k: &[f32]) -> Result<Vec<DiarizationRange>> {
        // Loaded per call and dropped after: Upload passes are rare, and the
        // engine holds hundreds of MB we don't want resident between notes.
        let mut sortformer = self.load_sortformer()?;
        let segments = sortformer
            .diarize(pcm_16k.to_vec(), 16_000, 1)
            .map_err(|e| anyhow::anyhow!("diarization failed: {e}"))
            .context("full-audio diarization pass")?;
        Ok(segments
            .iter()
            .map(|s| range_from_samples(s.speaker_id, s.start, s.end))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service_in(dir: &std::path::Path) -> Arc<DiarizationService> {
        DiarizationService::new(dir.join(SORTFORMER_MODEL_FILENAME))
    }

    #[test]
    fn samples_to_ms_converts_16k_sample_offsets() {
        assert_eq!(samples_to_ms(0), 0);
        assert_eq!(samples_to_ms(16_000), 1_000);
        assert_eq!(samples_to_ms(8_000), 500);
        // 93-minute recording offset: no overflow, exact division not required.
        assert_eq!(samples_to_ms(89_457_060), 5_591_066);
    }

    #[test]
    fn range_from_samples_maps_speaker_and_times() {
        let range = range_from_samples(2, 32_000, 48_000);
        assert_eq!(
            range,
            DiarizationRange {
                speaker_id: 2,
                start_ms: 2_000,
                end_ms: 3_000,
            }
        );
    }

    #[test]
    fn model_available_false_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!service_in(dir.path()).model_available());
    }

    #[test]
    fn model_available_false_for_zero_byte_placeholder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(SORTFORMER_MODEL_FILENAME), b"").unwrap();
        assert!(!service_in(dir.path()).model_available());
    }

    #[test]
    fn model_available_true_for_non_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(SORTFORMER_MODEL_FILENAME), b"onnx").unwrap();
        assert!(service_in(dir.path()).model_available());
    }

    #[test]
    fn diarize_errors_when_model_missing() {
        let dir = tempfile::tempdir().unwrap();
        let svc = service_in(dir.path());
        let err = svc.diarize(&[0.0; 16_000]).unwrap_err();
        assert!(err.to_string().contains("diarization model"), "{err}");
    }

    /// Manual smoke test against the real model. Run with:
    /// `SORTFORMER_MODEL=/path/to/diar_streaming_sortformer_4spk-v2.onnx cargo test -- --ignored diarize_real_model`
    #[test]
    #[ignore]
    fn diarize_real_model_returns_ranges_for_speech() {
        let model = std::env::var("SORTFORMER_MODEL").expect("set SORTFORMER_MODEL");
        let svc = DiarizationService::new(PathBuf::from(model));
        assert!(svc.model_available());
        // 10 s of alternating 200 Hz / 400 Hz tone bursts is enough for the
        // model to emit at least one span without needing a fixture WAV.
        let mut pcm = vec![0.0f32; 160_000];
        for (i, s) in pcm.iter_mut().enumerate() {
            let t = i as f32 / 16_000.0;
            let hz = if (t as usize) % 4 < 2 { 200.0 } else { 400.0 };
            *s = (t * hz * std::f32::consts::TAU).sin() * 0.4;
        }
        let ranges = svc.diarize(&pcm).unwrap();
        for r in &ranges {
            assert!(r.speaker_id < 4);
            assert!(r.end_ms > r.start_ms);
            assert!(r.end_ms <= 10_000);
        }
    }
}
