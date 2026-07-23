//! Shared post-capture transcription pipeline for Record, Upload, and Dictate.
//!
//! ADR-0003: Record, Dictate, and Upload are capture profiles of one system — any
//! new transcription capability belongs here, not in a single controller. This
//! module owns post-capture transcript result assembly:
//!
//! - ASR per source, hallucination filtering, and dual-source merge, with progress
//!   on one 0.0–1.0 scale across both sources (including an on-demand diarization
//!   pass for Upload).
//! - Anonymous speaker blocks: live-collected or on-demand diarization ranges
//!   aligned to ASR segments; channel labels for dual-source.
//! - Dictate's ASR-only result shape and final paste-ready text.

use crate::services::diarization::Diarizer;
use crate::services::model::ModelService;
use crate::services::output::{filter_hallucination_phrases, format_dictate_segments};
use crate::services::speaker_align::align_ranges_to_segments;
use crate::services::speaker_blocks::build_channel_blocks;
use crate::types::{DiarizationRange, Segment, SpeakerBlock, SpeakerChangeCut};
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

/// Anonymous speaker evidence for single-source Record/Upload.
pub enum SpeakerEvidenceInput<'a> {
    /// Record: ranges collected by the live diarization worker during capture.
    LiveRanges(&'a [DiarizationRange]),
    /// Upload: run one full-audio pass after ASR. Errors degrade to a plain
    /// transcript, never fail the note.
    DiarizeOnDemand(&'a dyn Diarizer),
}

/// Complete post-capture input. Audio is finalized 16 kHz mono PCM; capture and
/// durable output still live in controllers.
pub struct PostCaptureInput<'a> {
    pub profile: CaptureProfile,
    pub audio: CaptureAudio<'a>,
    pub model_path: &'a Path,
    pub speaker_evidence: Option<SpeakerEvidenceInput<'a>>,
    /// Persisted timeline enrichment (pitch/loudness jumps); no longer drives labels.
    pub speaker_change_cuts: &'a [SpeakerChangeCut],
    pub abort: Option<Arc<AtomicBool>>,
    /// Invoked once, when the model finishes loading for the first (mic) pass.
    pub on_model_loaded: Option<Box<dyn FnOnce() + Send>>,
}

#[derive(Debug, Default)]
pub struct TranscriptResult {
    pub segments: Vec<Segment>,
    pub speaker_blocks: Vec<SpeakerBlock>,
    pub speaker_change_cuts: Vec<SpeakerChangeCut>,
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
        let vad = self.vad_path_for_pcm(pass.pcm_16k.len())?;
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
    // An on-demand diarization pass follows ASR, so ASR progress is compressed
    // to leave visible headroom; live ranges arrive pre-computed and need none.
    let will_diarize_after_asr = !dual_source
        && input.profile.uses_speaker_evidence()
        && matches!(
            input.speaker_evidence,
            Some(SpeakerEvidenceInput::DiarizeOnDemand(_))
        );
    let asr_scale = if will_diarize_after_asr { 0.85 } else { 1.0 };
    let mut tail_progress = on_progress.clone();
    let mut scaled = on_progress;
    let asr_progress = move |p: f32| scaled(p * asr_scale);

    let mut on_model_loaded = input.on_model_loaded.take();

    // An on-demand diarization pass only needs the (already-decoded) mic PCM, and
    // Sortformer is a separate ONNX-backed engine loaded fresh per call — it shares
    // no lock or state with Whisper's inference_gate (see ticket 34's dual-source
    // finding, which is specific to whisper_full reentrancy, not diarization). So run
    // it on a scoped thread concurrently with the ASR pass instead of strictly after.
    let (segments, diarize_result) = if will_diarize_after_asr {
        let Some(SpeakerEvidenceInput::DiarizeOnDemand(diarizer)) = input.speaker_evidence else {
            unreachable!("will_diarize_after_asr implies DiarizeOnDemand")
        };
        let mic_pcm = input.audio.mic_pcm_16k;
        std::thread::scope(|scope| {
            let handle = scope.spawn(move || diarizer.diarize(mic_pcm));
            let segments = transcribe_capture_with_inference(
                inference,
                &input,
                &mut on_model_loaded,
                asr_progress,
            );
            let diarize_result = handle
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("diarization thread panicked")));
            (segments, Some(diarize_result))
        })
    } else {
        (
            transcribe_capture_with_inference(
                inference,
                &input,
                &mut on_model_loaded,
                asr_progress,
            ),
            None,
        )
    };
    let segments = segments?;
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

    let speaker_blocks = build_speaker_result(&input, &segments, dual_source, diarize_result);
    if will_diarize_after_asr {
        tail_progress(1.0);
    }

    Ok(TranscriptResult {
        segments,
        speaker_blocks,
        speaker_change_cuts: input.speaker_change_cuts.to_vec(),
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

/// `precomputed_diarize` carries the result of an on-demand diarization pass already
/// run concurrently with ASR (see the scoped-thread dispatch above); `None` when no
/// such pass was dispatched (`LiveRanges`/no-evidence cases still resolve inline here).
fn build_speaker_result(
    input: &PostCaptureInput<'_>,
    segments: &[Segment],
    dual_source: bool,
    precomputed_diarize: Option<Result<Vec<DiarizationRange>>>,
) -> Vec<SpeakerBlock> {
    if !input.profile.uses_speaker_evidence() {
        return Vec::new();
    }
    if dual_source {
        return build_channel_blocks(segments);
    }
    let ranges = match (&input.speaker_evidence, precomputed_diarize) {
        (None, _) => return Vec::new(),
        (Some(SpeakerEvidenceInput::LiveRanges(ranges)), _) => ranges.to_vec(),
        (Some(SpeakerEvidenceInput::DiarizeOnDemand(_)), Some(result)) => match result {
            Ok(ranges) => ranges,
            Err(err) => {
                tracing::warn!(error = %err, "diarization failed — saving plain transcript");
                return Vec::new();
            }
        },
        (Some(SpeakerEvidenceInput::DiarizeOnDemand(_)), None) => {
            unreachable!("DiarizeOnDemand always dispatches a precomputed pass")
        }
    };
    let blocks = align_ranges_to_segments(segments, &ranges);
    log_diarization_yield(&ranges, &blocks);
    blocks
}

/// Diagnostic for "the model only found N speakers" reports: distinguishes the
/// diarizer itself merging voices from alignment losing a speaker that WAS
/// detected but never won a Whisper segment (short interjections swallowed by
/// a coarser overlapping segment).
fn log_diarization_yield(ranges: &[crate::types::DiarizationRange], blocks: &[SpeakerBlock]) {
    let mut raw_speakers: Vec<u8> = ranges.iter().map(|r| r.speaker_id).collect();
    raw_speakers.sort_unstable();
    raw_speakers.dedup();

    let mut block_speakers: Vec<&str> = blocks
        .iter()
        .map(|b| b.label.as_str())
        .filter(|label| *label != crate::services::speaker_align::UNKNOWN_SPEAKER_LABEL)
        .collect();
    block_speakers.sort_unstable();
    block_speakers.dedup();

    tracing::info!(
        raw_speaker_count = raw_speakers.len(),
        raw_speaker_ids = ?raw_speakers,
        block_speaker_count = block_speakers.len(),
        block_speaker_labels = ?block_speakers,
        "diarization yield: raw speakers detected vs. speakers surviving into blocks"
    );
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

    fn range(speaker_id: u8, start_ms: u64, end_ms: u64) -> DiarizationRange {
        DiarizationRange {
            speaker_id,
            start_ms,
            end_ms,
        }
    }

    /// Full-pass diarizer double: canned result + call log for asserting the
    /// pass ran (or didn't) and saw the right PCM.
    struct FakeDiarizer {
        result: Mutex<Option<Result<Vec<DiarizationRange>>>>,
        seen_pcm_lens: Mutex<Vec<usize>>,
    }

    impl FakeDiarizer {
        fn returning(ranges: Vec<DiarizationRange>) -> Self {
            Self {
                result: Mutex::new(Some(Ok(ranges))),
                seen_pcm_lens: Mutex::new(Vec::new()),
            }
        }

        fn failing() -> Self {
            Self {
                result: Mutex::new(Some(Err(anyhow::anyhow!("onnx died")))),
                seen_pcm_lens: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<usize> {
            self.seen_pcm_lens.lock().unwrap().clone()
        }
    }

    impl Diarizer for FakeDiarizer {
        fn diarize(&self, pcm_16k: &[f32]) -> Result<Vec<DiarizationRange>> {
            self.seen_pcm_lens.lock().unwrap().push(pcm_16k.len());
            self.result
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Ok(Vec::new()))
        }
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
        speaker_evidence: Option<SpeakerEvidenceInput<'a>>,
        cuts: &'a [SpeakerChangeCut],
        abort: Option<Arc<AtomicBool>>,
    ) -> PostCaptureInput<'a> {
        PostCaptureInput {
            profile,
            audio: CaptureAudio {
                mic_pcm_16k: mic_pcm,
                speaker_pcm_16k: speaker_pcm,
            },
            model_path,
            speaker_evidence,
            speaker_change_cuts: cuts,
            abort,
            on_model_loaded: None,
        }
    }

    #[test]
    fn record_with_live_ranges_yields_aligned_speaker_blocks_and_cuts() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 6 * 16_000];
        let inference = FakeInference::new(vec![vec![
            Segment::new(0, 2_500, "first turn"),
            Segment::new(3_200, 5_800, "second turn"),
        ]]);
        let ranges = [range(0, 0, 2_900), range(1, 3_000, 6_000)];
        let cuts = [cut(3.0)];

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &pcm,
                None,
                Some(SpeakerEvidenceInput::LiveRanges(&ranges)),
                &cuts,
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments.len(), 2);
        let labels: Vec<&str> = result.speaker_blocks.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, vec!["Speaker 1", "Speaker 2"]);
        assert_eq!(result.speaker_change_cuts.len(), 1);
        assert!(!result.dual_source);
        assert_eq!(result.model_label, "model");
        assert!(result.dictate_text.is_none());
    }

    #[test]
    fn record_with_empty_live_ranges_marks_speech_as_other() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![vec![Segment::new(0, 900, "hello there")]]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &pcm,
                None,
                Some(SpeakerEvidenceInput::LiveRanges(&[])),
                &[],
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.speaker_blocks.len(), 1);
        assert_eq!(result.speaker_blocks[0].label, "Other");
    }

    #[test]
    fn record_without_evidence_keeps_plain_transcript() {
        let tmp = tempfile::tempdir().unwrap();
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
                None,
                &[],
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments[0].text, "plain transcript");
        assert!(result.speaker_blocks.is_empty());
    }

    #[test]
    fn upload_diarizes_on_demand_and_aligns() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("ggml-small.en-q5_1.bin");
        let pcm = vec![0.01f32; 4 * 16_000];
        let inference = FakeInference::new(vec![vec![Segment::new(0, 3_000, "upload text")]]);
        let diarizer = FakeDiarizer::returning(vec![range(2, 0, 4_000)]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Upload,
                &model_path,
                &pcm,
                None,
                Some(SpeakerEvidenceInput::DiarizeOnDemand(&diarizer)),
                &[],
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments[0].text, "upload text");
        assert_eq!(result.model_label, "small.en-q5_1");
        assert_eq!(result.speaker_blocks[0].label, "Speaker 3");
        // Exactly one full pass, over the full mic PCM.
        assert_eq!(diarizer.calls(), vec![4 * 16_000]);
    }

    #[test]
    fn upload_diarizer_failure_degrades_to_plain_transcript() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![vec![Segment::new(0, 900, "still here")]]);
        let diarizer = FakeDiarizer::failing();

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Upload,
                &model_path,
                &pcm,
                None,
                Some(SpeakerEvidenceInput::DiarizeOnDemand(&diarizer)),
                &[],
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments[0].text, "still here");
        assert!(result.speaker_blocks.is_empty());
    }

    #[test]
    fn on_demand_diarization_compresses_asr_progress_then_completes() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![vec![Segment::new(0, 900, "hello")]]);
        let diarizer = FakeDiarizer::returning(vec![range(0, 0, 900)]);
        let seen = Arc::new(Mutex::new(Vec::<f32>::new()));
        let sink = Arc::clone(&seen);

        run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Upload,
                &model_path,
                &pcm,
                None,
                Some(SpeakerEvidenceInput::DiarizeOnDemand(&diarizer)),
                &[],
                None,
            ),
            move |p| sink.lock().unwrap().push(p),
        )
        .unwrap();

        let seen = seen.lock().unwrap();
        let (last, head) = seen.split_last().expect("progress emitted");
        assert_eq!(*last, 1.0, "diarization completion emits 1.0");
        for p in head {
            assert!(*p <= 0.85, "ASR progress stays compressed, got {p}");
        }
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
            post_capture_input(
                CaptureProfile::Dictate,
                &model_path,
                &pcm,
                None,
                None,
                &[],
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.dictate_text.as_deref(), Some("hello world"));
        assert!(result.speaker_blocks.is_empty());
    }

    #[test]
    fn dual_source_returns_channel_blocks_and_never_diarizes() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let mic_pcm = vec![0.01f32; 16_000];
        let speaker_pcm = vec![0.02f32; 16_000];
        let inference = FakeInference::new(vec![
            vec![Segment::new(0, 600, "mic line")],
            vec![Segment::new(700, 1_200, "speaker line")],
        ]);
        let diarizer = FakeDiarizer::returning(vec![range(0, 0, 1_200)]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Record,
                &model_path,
                &mic_pcm,
                Some(&speaker_pcm),
                Some(SpeakerEvidenceInput::DiarizeOnDemand(&diarizer)),
                &[],
                None,
            ),
            |_p| {},
        )
        .unwrap();

        assert!(result.dual_source);
        assert_eq!(result.segments.len(), 2);
        let labels: Vec<&str> = result.speaker_blocks.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, vec!["In", "Out"]);
        assert!(diarizer.calls().is_empty(), "dual-source must not diarize");
    }

    #[test]
    fn empty_asr_result_keeps_controller_observable_empty_result() {
        let tmp = tempfile::tempdir().unwrap();
        let model_path = tmp.path().join("model.bin");
        let pcm = vec![0.01f32; 16_000];
        let inference = FakeInference::new(vec![Vec::new()]);

        let result = run_post_capture_transcription_with_inference(
            &inference,
            post_capture_input(
                CaptureProfile::Dictate,
                &model_path,
                &pcm,
                None,
                None,
                &[],
                None,
            ),
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
                &[],
                Some(abort),
            ),
            |_p| {},
        )
        .unwrap();

        assert!(result.segments.is_empty());
        assert_eq!(inference.call_count(), 1);
    }
}
