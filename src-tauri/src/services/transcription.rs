//! Shared post-capture transcription pipeline for the Record and Upload paths.
//!
//! ADR-0003: Record, Dictate, and Upload are capture profiles of one system — any
//! new transcription capability belongs here, not in a single controller. This
//! module owns the two stages both paths were duplicating:
//!
//! - `transcribe_capture` — Whisper per source, hallucination filtering, and the
//!   dual-source merge, with progress on one 0.0–1.0 scale across both sources.
//! - `analyze_capture_speakers` — chunk-based speaker evidence (voice-turn chunks,
//!   session speaker centroids, speaker blocks) derived together so the three
//!   collections always reference each other consistently.

use crate::services::model::ModelService;
use crate::services::output::filter_hallucination_phrases;
use crate::services::speaker_chunks::{
    analyze_chunks, build_blocks_from_chunks, build_session_speakers, score_chunks,
};
use crate::services::voiceprint::VoiceprintService;
use crate::types::{Segment, SessionSpeaker, SpeakerBlock, SpeakerChangeCut, SpeakerChunk};
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Audio captured for one transcription pass, already resampled to 16 kHz mono.
pub struct CaptureAudio<'a> {
    pub mic_pcm_16k: &'a [f32],
    pub speaker_pcm_16k: Option<&'a [f32]>,
}

/// Everything Whisper needs besides the audio itself.
pub struct TranscribeOptions<'a> {
    pub model_path: &'a Path,
    /// Caller workflow for failure diagnostics — "scribe" or "transcribe"; the
    /// per-source suffix ("/mic", "/speaker") is appended here.
    pub source: &'a str,
    pub abort: Option<Arc<AtomicBool>>,
    /// Invoked once, when the model finishes loading for the first (mic) pass.
    pub on_model_loaded: Option<Box<dyn FnOnce() + Send>>,
}

/// Transcribe one capture. Dual-source: mic pass maps to progress 0.0–0.5 and the
/// speaker pass to 0.5–1.0; both passes are hallucination-filtered before the merge.
/// Returns an empty segment list when the caller aborts between passes.
pub fn transcribe_capture<F>(
    model: &ModelService,
    audio: CaptureAudio<'_>,
    opts: TranscribeOptions<'_>,
    on_progress: F,
) -> Result<Vec<Segment>>
where
    F: FnMut(f32) + Send + Clone + 'static,
{
    let mic_vad = model.vad_path_for_pcm(audio.mic_pcm_16k.len());
    let mic_source = format!("{}/mic", opts.source);
    if let Some(speaker_pcm) = audio.speaker_pcm_16k {
        let mut mic_progress = on_progress.clone();
        let mic_segments = model.transcribe_pcm_with_progress(
            opts.model_path,
            audio.mic_pcm_16k,
            mic_vad.as_deref(),
            opts.abort.clone(),
            &mic_source,
            move |p| mic_progress(p * 0.5),
            opts.on_model_loaded,
        )?;
        // Skip the second (speaker) pass entirely if the user aborted during the mic pass.
        if opts
            .abort
            .as_ref()
            .is_some_and(|abort| abort.load(Ordering::SeqCst))
        {
            return Ok(Vec::new());
        }
        let speaker_vad = model.vad_path_for_pcm(speaker_pcm.len());
        let mut speaker_progress = on_progress;
        let speaker_segments = model.transcribe_pcm_with_progress(
            opts.model_path,
            speaker_pcm,
            speaker_vad.as_deref(),
            opts.abort,
            &format!("{}/speaker", opts.source),
            move |p| speaker_progress(0.5 + p * 0.5),
            None,
        )?;
        let mic_segments = filter_hallucination_phrases(&mic_segments);
        let speaker_segments = filter_hallucination_phrases(&speaker_segments);
        Ok(model.merge_dual_source(&mic_segments, &speaker_segments))
    } else {
        model
            .transcribe_pcm_with_progress(
                opts.model_path,
                audio.mic_pcm_16k,
                mic_vad.as_deref(),
                opts.abort,
                &mic_source,
                on_progress,
                opts.on_model_loaded,
            )
            .map(|segments| filter_hallucination_phrases(&segments))
    }
}

/// Chunk-derived speaker evidence for one single-source (mic) capture.
#[derive(Debug, Default)]
pub struct SpeakerEvidence {
    pub speaker_blocks: Vec<SpeakerBlock>,
    pub speaker_chunks: Vec<SpeakerChunk>,
    pub session_speakers: Vec<SessionSpeaker>,
}

/// Derive the three chunk-based speaker collections together: blocks reference chunk
/// ids and session speakers aggregate the same chunks, so producing them separately
/// risks inconsistent evidence.
pub fn analyze_capture_speakers(
    pcm_16k: &[f32],
    sample_rate: u32,
    cuts: &[SpeakerChangeCut],
    voiceprint: &VoiceprintService,
    profile_threshold: f32,
    segments: &[Segment],
) -> SpeakerEvidence {
    let mut speaker_chunks =
        analyze_chunks(pcm_16k, sample_rate, cuts, voiceprint, profile_threshold);
    let session_speakers = build_session_speakers(&speaker_chunks);
    score_chunks(&mut speaker_chunks, &session_speakers);
    let speaker_blocks = build_blocks_from_chunks(segments, &speaker_chunks);
    SpeakerEvidence {
        speaker_blocks,
        speaker_chunks,
        session_speakers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CutReason;
    use std::collections::BTreeSet;

    fn cut(time_s: f32) -> SpeakerChangeCut {
        SpeakerChangeCut {
            time_s,
            end_s: time_s,
            score: 1.0,
            reasons: BTreeSet::from([CutReason::Pitch]),
        }
    }

    fn test_voiceprint(dir: &Path) -> VoiceprintService {
        VoiceprintService::new(&dir.join("missing-model.onnx"), &dir.join("profiles"), 0.75)
            .expect("voiceprint service")
    }

    #[test]
    fn analyze_capture_speakers_derives_consistent_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let voiceprint = test_voiceprint(tmp.path());
        // 6 s of quiet audio at 16 kHz with one cut at 3 s → two voice-turn chunks.
        let pcm = vec![0.01f32; 6 * 16_000];
        let segments = vec![
            Segment::new(0, 2_500, "first turn"),
            Segment::new(3_200, 5_800, "second turn"),
        ];
        let evidence =
            analyze_capture_speakers(&pcm, 16_000, &[cut(3.0)], &voiceprint, 0.75, &segments);

        assert_eq!(evidence.speaker_chunks.len(), 2);
        assert_eq!(evidence.speaker_chunks[0].start_ms, 0);
        assert_eq!(evidence.speaker_chunks[0].end_ms, 3_000);
        assert_eq!(evidence.speaker_chunks[1].start_ms, 3_000);
        assert_eq!(evidence.speaker_chunks[1].end_ms, 6_000);
        // Every block must reference a chunk that exists in the same evidence set.
        for block in &evidence.speaker_blocks {
            let chunk_id = block.chunk_id.as_deref().expect("block links a chunk");
            assert!(
                evidence
                    .speaker_chunks
                    .iter()
                    .any(|chunk| chunk.id == chunk_id),
                "block references unknown chunk {chunk_id}"
            );
        }
        // No embeddings (model file missing) → no clean chunks → no session speakers.
        assert!(evidence.session_speakers.is_empty());
    }

    #[test]
    fn analyze_capture_speakers_empty_audio_yields_empty_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let voiceprint = test_voiceprint(tmp.path());
        let evidence = analyze_capture_speakers(&[], 16_000, &[], &voiceprint, 0.75, &[]);
        assert!(evidence.speaker_blocks.is_empty());
        assert!(evidence.speaker_chunks.is_empty());
        assert!(evidence.session_speakers.is_empty());
    }

    #[test]
    fn transcribe_capture_errors_when_model_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let model = ModelService::new(tmp.path().to_path_buf());
        let pcm = vec![0.0f32; 16_000];
        let result = transcribe_capture(
            &model,
            CaptureAudio {
                mic_pcm_16k: &pcm,
                speaker_pcm_16k: None,
            },
            TranscribeOptions {
                model_path: &tmp.path().join("missing.bin"),
                source: "test",
                abort: None,
                on_model_loaded: None,
            },
            |_p| {},
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
