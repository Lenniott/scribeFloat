//! Shared post-capture transcription pipeline for Record, Upload, and Dictate.
//!
//! ADR-0003: Record, Dictate, and Upload are capture profiles of one system — any
//! new transcription capability belongs here, not in a single controller. This
//! module owns post-capture transcript result assembly:
//!
//! - ASR per source, hallucination filtering, and dual-source merge, with progress
//!   on one 0.0–1.0 scale across both sources.
//! - Speaker evidence assembly for Record and Upload, including legacy chunk fields.
//! - Dictate's ASR-only result shape and final paste-ready text.

use crate::services::model::ModelService;
use crate::services::output::{filter_hallucination_phrases, format_dictate_segments};
use crate::services::speaker_blocks::build_speaker_blocks;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureProfile {
    Record,
    Upload,
    Dictate,
}

impl CaptureProfile {
    fn source_name(self) -> &'static str {
        match self {
            CaptureProfile::Record => "scribe",
            CaptureProfile::Upload => "transcribe",
            CaptureProfile::Dictate => "dictate",
        }
    }

    fn uses_speaker_evidence(self) -> bool {
        matches!(self, CaptureProfile::Record | CaptureProfile::Upload)
    }
}

/// Speaker analysis dependencies for profiles that produce labelled transcript blocks.
pub struct SpeakerAnalysisInput<'a> {
    pub voiceprint: &'a VoiceprintService,
    pub profile_threshold: f32,
    pub speaker_change_cuts: &'a [SpeakerChangeCut],
}

/// Complete post-capture input. Audio is finalized 16 kHz mono PCM; capture and
/// durable output still live in controllers.
pub struct PostCaptureInput<'a> {
    pub profile: CaptureProfile,
    pub audio: CaptureAudio<'a>,
    pub model_path: &'a Path,
    pub speaker_analysis: Option<SpeakerAnalysisInput<'a>>,
    pub abort: Option<Arc<AtomicBool>>,
    /// Invoked once, when the model finishes loading for the first (mic) pass.
    pub on_model_loaded: Option<Box<dyn FnOnce() + Send>>,
}

#[derive(Debug, Default)]
pub struct TranscriptResult {
    pub segments: Vec<Segment>,
    pub speaker_blocks: Vec<SpeakerBlock>,
    pub speaker_change_cuts: Vec<SpeakerChangeCut>,
    pub speaker_chunks: Vec<SpeakerChunk>,
    pub session_speakers: Vec<SessionSpeaker>,
    pub dual_source: bool,
    pub model_label: String,
    pub dictate_text: Option<String>,
}

struct TranscriptionPass<'a> {
    model_path: &'a Path,
    pcm_16k: &'a [f32],
    abort: Option<Arc<AtomicBool>>,
    source: String,
    on_model_loaded: Option<Box<dyn FnOnce() + Send>>,
}

trait SpeechInference {
    fn transcribe_pass(
        &self,
        pass: TranscriptionPass<'_>,
        on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<Segment>>;

    fn merge_dual_source(
        &self,
        mic_segments: &[Segment],
        speaker_segments: &[Segment],
    ) -> Vec<Segment>;
}

impl SpeechInference for ModelService {
    fn transcribe_pass(
        &self,
        pass: TranscriptionPass<'_>,
        on_progress: Box<dyn FnMut(f32) + Send>,
    ) -> Result<Vec<Segment>> {
        let vad = self.vad_path_for_pcm(pass.pcm_16k.len());
        self.transcribe_pcm_with_progress(
            pass.model_path,
            pass.pcm_16k,
            vad.as_deref(),
            pass.abort,
            &pass.source,
            on_progress,
            pass.on_model_loaded,
        )
    }

    fn merge_dual_source(
        &self,
        mic_segments: &[Segment],
        speaker_segments: &[Segment],
    ) -> Vec<Segment> {
        self.merge_dual_source(mic_segments, speaker_segments)
    }
}

pub fn run_post_capture_transcription<F>(
    model: &ModelService,
    input: PostCaptureInput<'_>,
    on_progress: F,
) -> Result<TranscriptResult>
where
    F: FnMut(f32) + Send + Clone + 'static,
{
    run_post_capture_transcription_with_inference(model, input, on_progress)
}

/// Chunk-derived speaker evidence for one single-source (mic) capture.
#[derive(Debug, Default)]
struct SpeakerEvidence {
    pub speaker_blocks: Vec<SpeakerBlock>,
    pub speaker_chunks: Vec<SpeakerChunk>,
    pub session_speakers: Vec<SessionSpeaker>,
}

/// Derive the three chunk-based speaker collections together: blocks reference chunk
/// ids and session speakers aggregate the same chunks, so producing them separately
/// risks inconsistent evidence.
fn analyze_capture_speakers(
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

fn run_post_capture_transcription_with_inference<F, I>(
    inference: &I,
    mut input: PostCaptureInput<'_>,
    on_progress: F,
) -> Result<TranscriptResult>
where
    F: FnMut(f32) + Send + Clone + 'static,
    I: SpeechInference,
{
    let dual_source = input.audio.speaker_pcm_16k.is_some();
    let mut on_model_loaded = input.on_model_loaded.take();
    let segments =
        transcribe_capture_with_inference(inference, &input, &mut on_model_loaded, on_progress)?;
    let model_label = model_label(input.model_path);
    if segments.is_empty() {
        return Ok(TranscriptResult {
            segments,
            dual_source,
            model_label,
            ..TranscriptResult::default()
        });
    }

    if input.profile == CaptureProfile::Dictate {
        let dictate_text = Some(format_dictate_segments(&segments));
        return Ok(TranscriptResult {
            segments,
            dual_source,
            model_label,
            dictate_text,
            ..TranscriptResult::default()
        });
    }

    let (speaker_blocks, speaker_chunks, session_speakers, speaker_change_cuts) =
        build_speaker_result(&input, &segments, dual_source);

    Ok(TranscriptResult {
        segments,
        speaker_blocks,
        speaker_change_cuts,
        speaker_chunks,
        session_speakers,
        dual_source,
        model_label,
        dictate_text: None,
    })
}

fn transcribe_capture_with_inference<F, I>(
    inference: &I,
    input: &PostCaptureInput<'_>,
    on_model_loaded: &mut Option<Box<dyn FnOnce() + Send>>,
    on_progress: F,
) -> Result<Vec<Segment>>
where
    F: FnMut(f32) + Send + Clone + 'static,
    I: SpeechInference,
{
    let source = input.profile.source_name();
    let mic_source = format!("{source}/mic");
    if let Some(speaker_pcm) = input.audio.speaker_pcm_16k {
        let mut mic_progress = on_progress.clone();
        let mic_segments = inference.transcribe_pass(
            TranscriptionPass {
                model_path: input.model_path,
                pcm_16k: input.audio.mic_pcm_16k,
                abort: input.abort.clone(),
                source: mic_source,
                on_model_loaded: on_model_loaded.take(),
            },
            Box::new(move |p: f32| mic_progress(p * 0.5)),
        )?;
        if input
            .abort
            .as_ref()
            .is_some_and(|abort| abort.load(Ordering::SeqCst))
        {
            return Ok(Vec::new());
        }

        let mut speaker_progress = on_progress;
        let speaker_segments = inference.transcribe_pass(
            TranscriptionPass {
                model_path: input.model_path,
                pcm_16k: speaker_pcm,
                abort: input.abort.clone(),
                source: format!("{source}/speaker"),
                on_model_loaded: None,
            },
            Box::new(move |p: f32| speaker_progress(0.5 + p * 0.5)),
        )?;
        let mic_segments = filter_hallucination_phrases(&mic_segments);
        let speaker_segments = filter_hallucination_phrases(&speaker_segments);
        Ok(inference.merge_dual_source(&mic_segments, &speaker_segments))
    } else {
        inference
            .transcribe_pass(
                TranscriptionPass {
                    model_path: input.model_path,
                    pcm_16k: input.audio.mic_pcm_16k,
                    abort: input.abort.clone(),
                    source: mic_source,
                    on_model_loaded: on_model_loaded.take(),
                },
                Box::new(on_progress),
            )
            .map(|segments| filter_hallucination_phrases(&segments))
    }
}

fn build_speaker_result(
    input: &PostCaptureInput<'_>,
    segments: &[Segment],
    dual_source: bool,
) -> (
    Vec<SpeakerBlock>,
    Vec<SpeakerChunk>,
    Vec<SessionSpeaker>,
    Vec<SpeakerChangeCut>,
) {
    let Some(analysis) = input.speaker_analysis.as_ref() else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };
    if !input.profile.uses_speaker_evidence() {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    }

    if dual_source {
        let blocks = label_speaker_blocks(
            segments,
            input.audio.mic_pcm_16k,
            analysis.voiceprint,
            analysis.profile_threshold,
            true,
        );
        return (
            blocks,
            Vec::new(),
            Vec::new(),
            analysis.speaker_change_cuts.to_vec(),
        );
    }

    let evidence = analyze_capture_speakers(
        input.audio.mic_pcm_16k,
        crate::services::audio::WHISPER_SAMPLE_RATE,
        analysis.speaker_change_cuts,
        analysis.voiceprint,
        analysis.profile_threshold,
        segments,
    );
    let fallback_blocks = || {
        label_speaker_blocks(
            segments,
            input.audio.mic_pcm_16k,
            analysis.voiceprint,
            analysis.profile_threshold,
            false,
        )
    };
    let blocks = if evidence
        .speaker_blocks
        .iter()
        .any(|block| block.label != "Other")
    {
        evidence.speaker_blocks
    } else {
        fallback_blocks()
    };
    (
        blocks,
        evidence.speaker_chunks,
        evidence.session_speakers,
        analysis.speaker_change_cuts.to_vec(),
    )
}

fn label_speaker_blocks(
    segments: &[Segment],
    session_pcm: &[f32],
    voiceprint: &VoiceprintService,
    threshold: f32,
    dual_source: bool,
) -> Vec<SpeakerBlock> {
    match build_speaker_blocks(
        segments,
        session_pcm,
        crate::services::audio::WHISPER_SAMPLE_RATE,
        voiceprint,
        threshold,
        dual_source,
    ) {
        Ok(blocks) => blocks,
        Err(err) => {
            tracing::warn!(error = %err, "speaker labelling skipped");
            Vec::new()
        }
    }
}

fn model_label(model_path: &Path) -> String {
    model_path
        .file_stem()
        .map(|s| s.to_string_lossy().replace("ggml-", ""))
        .unwrap_or_else(|| "model".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CutReason, SegmentSource};
    use std::collections::{BTreeSet, VecDeque};
    use std::sync::Mutex;

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

    struct FakeInference {
        passes: Mutex<VecDeque<Vec<Segment>>>,
        calls: Mutex<Vec<String>>,
        abort_after_mic: bool,
    }

    impl FakeInference {
        fn new(passes: Vec<Vec<Segment>>) -> Self {
            Self {
                passes: Mutex::new(VecDeque::from(passes)),
                calls: Mutex::new(Vec::new()),
                abort_after_mic: false,
            }
        }

        fn aborting_after_mic(passes: Vec<Vec<Segment>>) -> Self {
            Self {
                abort_after_mic: true,
                ..Self::new(passes)
            }
        }

        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl SpeechInference for FakeInference {
        fn transcribe_pass(
            &self,
            pass: TranscriptionPass<'_>,
            mut on_progress: Box<dyn FnMut(f32) + Send>,
        ) -> Result<Vec<Segment>> {
            self.calls.lock().unwrap().push(pass.source.clone());
            if let Some(cb) = pass.on_model_loaded {
                cb();
            }
            on_progress(1.0);
            let segments = self.passes.lock().unwrap().pop_front().unwrap_or_default();
            if self.abort_after_mic && pass.source.ends_with("/mic") {
                if let Some(abort) = pass.abort {
                    abort.store(true, Ordering::SeqCst);
                }
            }
            Ok(segments)
        }

        fn merge_dual_source(
            &self,
            mic_segments: &[Segment],
            speaker_segments: &[Segment],
        ) -> Vec<Segment> {
            let mut out = Vec::new();
            out.extend(mic_segments.iter().cloned().map(|mut segment| {
                segment.source = Some(SegmentSource::Mic);
                segment
            }));
            out.extend(speaker_segments.iter().cloned().map(|mut segment| {
                segment.source = Some(SegmentSource::Speaker);
                segment
            }));
            out.sort_by_key(|segment| segment.start_ms);
            out
        }
    }

    fn post_capture_input<'a>(
        profile: CaptureProfile,
        model_path: &'a Path,
        mic_pcm: &'a [f32],
        speaker_pcm: Option<&'a [f32]>,
        speaker_analysis: Option<SpeakerAnalysisInput<'a>>,
        abort: Option<Arc<AtomicBool>>,
    ) -> PostCaptureInput<'a> {
        PostCaptureInput {
            profile,
            audio: CaptureAudio {
                mic_pcm_16k: mic_pcm,
                speaker_pcm_16k: speaker_pcm,
            },
            model_path,
            speaker_analysis,
            abort,
            on_model_loaded: None,
        }
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
    fn record_single_source_returns_segments_and_legacy_speaker_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let voiceprint = test_voiceprint(tmp.path());
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 6 * 16_000];
        let inference = FakeInference::new(vec![vec![
            Segment::new(0, 2_500, "first turn"),
            Segment::new(3_200, 5_800, "second turn"),
        ]]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &pcm,
                None,
                Some(SpeakerAnalysisInput {
                    voiceprint: &voiceprint,
                    profile_threshold: 0.75,
                    speaker_change_cuts: &[cut(3.0)],
                }),
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.speaker_chunks.len(), 2);
        assert_eq!(result.speaker_change_cuts.len(), 1);
        assert!(!result.dual_source);
        assert_eq!(result.model_label, "model");
        assert!(result.dictate_text.is_none());
    }

    #[test]
    fn upload_single_source_uses_same_result_shape_as_record() {
        let tmp = tempfile::tempdir().unwrap();
        let voiceprint = test_voiceprint(tmp.path());
        let model_path = tmp.path().join("ggml-small.en-q5_1.bin");
        let pcm = vec![0.01f32; 4 * 16_000];
        let inference = FakeInference::new(vec![vec![Segment::new(0, 3_000, "upload text")]]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Upload,
                &model_path,
                &pcm,
                None,
                Some(SpeakerAnalysisInput {
                    voiceprint: &voiceprint,
                    profile_threshold: 0.75,
                    speaker_change_cuts: &[],
                }),
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments[0].text, "upload text");
        assert_eq!(result.model_label, "small.en-q5_1");
        assert!(!result.dual_source);
        assert!(result.dictate_text.is_none());
    }

    #[test]
    fn dictate_returns_asr_only_result_with_final_text() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("dictate.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![vec![
            Segment::new(0, 500, " hello "),
            Segment::new(500, 900, "world"),
        ]]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(CaptureProfile::Dictate, &model_path, &pcm, None, None, None),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.dictate_text.as_deref(), Some("hello world"));
        assert!(result.speaker_blocks.is_empty());
        assert!(result.speaker_chunks.is_empty());
    }

    #[test]
    fn dual_source_returns_channel_blocks_and_skips_identity_evidence() {
        let tmp = tempfile::tempdir().unwrap();
        let voiceprint = test_voiceprint(tmp.path());
        let model_path = tmp.path().join("model.bin");
        let mic_pcm = vec![0.01f32; 16_000];
        let speaker_pcm = vec![0.02f32; 16_000];
        let inference = FakeInference::new(vec![
            vec![Segment::new(0, 600, "mic line")],
            vec![Segment::new(700, 1_200, "speaker line")],
        ]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &mic_pcm,
                Some(&speaker_pcm),
                Some(SpeakerAnalysisInput {
                    voiceprint: &voiceprint,
                    profile_threshold: 0.75,
                    speaker_change_cuts: &[],
                }),
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert!(result.dual_source);
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.speaker_blocks.len(), 2);
        assert!(result.speaker_chunks.is_empty());
        assert!(result.session_speakers.is_empty());
    }

    #[test]
    fn empty_asr_result_keeps_controller_observable_empty_result() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![Vec::new()]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(CaptureProfile::Dictate, &model_path, &pcm, None, None, None),
            |_p| {},
        )
        .unwrap();

        assert!(result.segments.is_empty());
        assert!(result.dictate_text.is_none());
        assert!(!result.dual_source);
    }

    #[test]
    fn abort_between_dual_source_passes_skips_speaker_pass() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let mic_pcm = vec![0.01f32; 16_000];
        let speaker_pcm = vec![0.02f32; 16_000];
        let abort = Arc::new(AtomicBool::new(false));
        let inference = FakeInference::aborting_after_mic(vec![
            vec![Segment::new(0, 600, "mic line")],
            vec![Segment::new(700, 1_200, "speaker line")],
        ]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &mic_pcm,
                Some(&speaker_pcm),
                None,
                Some(abort),
            ),
            |_p| {},
        )
        .unwrap();

        assert!(result.segments.is_empty());
        assert_eq!(inference.call_count(), 1);
    }

    #[test]
    fn speaker_labelling_fallback_keeps_plain_transcript_when_profiles_are_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let voiceprint = test_voiceprint(tmp.path());
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![vec![Segment::new(0, 900, "plain transcript")]]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &pcm,
                None,
                Some(SpeakerAnalysisInput {
                    voiceprint: &voiceprint,
                    profile_threshold: 0.75,
                    speaker_change_cuts: &[],
                }),
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments[0].text, "plain transcript");
        assert!(result.speaker_blocks.is_empty());
    }
}
