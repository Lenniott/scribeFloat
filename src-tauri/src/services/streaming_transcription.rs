//! Capture-time ASR shared by capture profiles. WAV remains the recovery source.
//! Silence closes disjoint audio spans; stored segments retain absolute times.
use super::{
    audio::{pcm_as_saved_wav, Pcm16kTap},
    model::ModelService,
    output::filter_hallucination_phrases,
};
use crate::types::Segment;
use anyhow::{anyhow, ensure, Result};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use whisper_rs::{WhisperVadContext, WhisperVadContextParams};

const RATE: usize = 16_000;
const STEP: usize = 8_192; // 512 ms, aligned to Silero's 512-sample frames.
const WINDOW: usize = STEP * 4;
// Keep a full Whisper context before splitting. Small phrases changed words in
// fixture comparisons; the win sought here is overlapping long capture with ASR.
const MIN_PHRASE: usize = RATE * 30;
const MAX_PENDING: usize = RATE * 60;
const QUIET_FRAMES: usize = 16; // 512 ms of confirmed silence.
const PACKET: usize = 1_280;

/// Runs Silero on overlapping context windows. Its public API resets recurrent
/// state each call, so retain two seconds of context and consume only new scores.
pub struct PhraseSegmenter {
    vad: WhisperVadContext,
    buffer: PhraseBuffer,
}

#[derive(Default)]
struct PhraseBuffer {
    pending: Vec<f32>,
    context: Vec<f32>,
    scored: usize,
    quiet_frames: usize,
    speech_seen: bool,
    offset: usize,
}

pub struct AudioPhrase {
    pub pcm: Vec<f32>,
    pub start_sample: usize,
    pub ready_sample: usize,
}

impl PhraseSegmenter {
    pub fn new(vad_path: &std::path::Path) -> Result<Self> {
        let mut params = WhisperVadContextParams::default();
        params.set_use_gpu(false);
        params.set_n_threads(1);
        Ok(Self {
            vad: WhisperVadContext::new(
                vad_path
                    .to_str()
                    .ok_or_else(|| anyhow!("invalid VAD path"))?,
                params,
            )?,
            buffer: PhraseBuffer::default(),
        })
    }

    pub fn push(&mut self, pcm: &[f32]) -> Result<Vec<AudioPhrase>> {
        self.buffer.push(pcm, |context| {
            self.vad.detect_speech(context)?;
            Ok(self.vad.probabilities().to_vec())
        })
    }

    pub fn finish(self) -> Option<AudioPhrase> {
        self.buffer.finish()
    }
}

impl PhraseBuffer {
    fn push(
        &mut self,
        pcm: &[f32],
        mut detect: impl FnMut(&[f32]) -> Result<Vec<f32>>,
    ) -> Result<Vec<AudioPhrase>> {
        let mut phrases = Vec::new();
        for part in pcm.chunks(PACKET) {
            self.pending.extend_from_slice(part);
            ensure!(
                self.pending.len() <= MAX_PENDING,
                "no safe pause within live ASR buffer limit"
            );
            while self.pending.len() - self.scored >= STEP {
                self.context
                    .extend_from_slice(&self.pending[self.scored..self.scored + STEP]);
                if self.context.len() > WINDOW {
                    self.context.drain(..self.context.len() - WINDOW);
                }
                let probabilities = detect(&self.context)?;
                for &p in &probabilities[probabilities.len().saturating_sub(STEP / 512)..] {
                    if p >= 0.5 {
                        self.speech_seen = true;
                        self.quiet_frames = 0;
                    } else {
                        self.quiet_frames += 1;
                    }
                }
                self.scored += STEP;
                if self.speech_seen
                    && self.scored >= MIN_PHRASE
                    && self.quiet_frames >= QUIET_FRAMES
                {
                    let tail = self.pending.split_off(self.scored);
                    let phrase = AudioPhrase {
                        pcm: std::mem::replace(&mut self.pending, tail),
                        start_sample: self.offset,
                        ready_sample: self.offset + self.scored,
                    };
                    self.offset += self.scored;
                    self.scored = 0;
                    self.speech_seen = false;
                    self.quiet_frames = 0;
                    phrases.push(phrase);
                }
            }
        }
        Ok(phrases)
    }

    pub fn finish(self) -> Option<AudioPhrase> {
        (!self.pending.is_empty()).then(|| AudioPhrase {
            ready_sample: self.offset + self.pending.len(),
            pcm: self.pending,
            start_sample: self.offset,
        })
    }
}

enum Message {
    Audio(Vec<f32>),
    Finish,
}

#[derive(Debug)]
pub struct LiveTranscript {
    pub segments: Vec<Segment>,
    pub audio_samples: usize,
}

/// Bounded nonblocking tap: any overflow invalidates live results, so the caller
/// can replay the complete WAV instead of silently losing words. Drop cancels;
/// finish joins. No worker writes Notes, pastes, or emits capture state changes.
pub struct StreamingTranscription {
    sender: mpsc::SyncSender<Message>,
    cancel: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<LiveTranscript>>>,
    finished: bool,
}

impl StreamingTranscription {
    pub fn start(model: Arc<ModelService>, model_path: PathBuf) -> Result<Self> {
        Self::spawn(move |receiver, cancelled, invalid| {
            let vad = model
                .vad_path_for_pcm(RATE * 2)?
                .ok_or_else(|| anyhow!("VAD unavailable"))?;
            let segmenter = PhraseSegmenter::new(&vad)?;
            let abort = Arc::clone(&cancelled);
            run_worker(receiver, cancelled, invalid, segmenter, |phrase| {
                transcribe_phrase(&model, &model_path, phrase, Arc::clone(&abort))
            })
        })
    }

    fn spawn(
        work: impl FnOnce(
                mpsc::Receiver<Message>,
                Arc<AtomicBool>,
                Arc<AtomicBool>,
            ) -> Result<LiveTranscript>
            + Send
            + 'static,
    ) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(512);
        let cancel = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::clone(&cancel);
        let invalid = Arc::clone(&failed);
        let worker = thread::Builder::new()
            .name("capture-asr".into())
            .spawn(move || work(receiver, cancelled, invalid))?;
        Ok(Self {
            sender,
            cancel,
            failed,
            worker: Some(worker),
            finished: false,
        })
    }

    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    pub fn tap(&self) -> Pcm16kTap {
        let sender = self.sender.clone();
        let failed = Arc::clone(&self.failed);
        let cancel = Arc::clone(&self.cancel);
        Arc::new(move |pcm| {
            if failed.load(Ordering::Relaxed) || cancel.load(Ordering::Relaxed) {
                return;
            }
            for packet in pcm.chunks(PACKET) {
                if sender
                    .try_send(Message::Audio(pcm_as_saved_wav(packet)))
                    .is_err()
                {
                    failed.store(true, Ordering::SeqCst);
                    break;
                }
            }
        })
    }

    pub fn finish(mut self) -> Result<LiveTranscript> {
        // Even if sending fails, join to reap the worker and preserve its error.
        let _ = self.sender.send(Message::Finish);
        let result = self
            .worker
            .take()
            .expect("worker consumed once")
            .join()
            .map_err(|_| anyhow!("live ASR worker panicked"));
        self.finished = true;
        result?
    }
}

trait AudioSegmenter {
    fn push(&mut self, pcm: &[f32]) -> Result<Vec<AudioPhrase>>;
    fn finish(self) -> Option<AudioPhrase>;
}

impl AudioSegmenter for PhraseSegmenter {
    fn push(&mut self, pcm: &[f32]) -> Result<Vec<AudioPhrase>> {
        self.push(pcm)
    }
    fn finish(self) -> Option<AudioPhrase> {
        self.finish()
    }
}

fn run_worker(
    receiver: mpsc::Receiver<Message>,
    cancelled: Arc<AtomicBool>,
    invalid: Arc<AtomicBool>,
    mut segmenter: impl AudioSegmenter,
    mut transcribe: impl FnMut(AudioPhrase) -> Result<Vec<Segment>>,
) -> Result<LiveTranscript> {
    let mut segments = Vec::new();
    let mut audio_samples = 0;
    loop {
        ensure!(!cancelled.load(Ordering::SeqCst), "live ASR cancelled");
        ensure!(!invalid.load(Ordering::SeqCst), "live ASR queue overflowed");
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Message::Audio(pcm)) => {
                audio_samples += pcm.len();
                for phrase in segmenter.push(&pcm)? {
                    ensure!(!cancelled.load(Ordering::SeqCst), "live ASR cancelled");
                    segments.extend(transcribe(phrase)?);
                }
            }
            Ok(Message::Finish) => {
                if let Some(phrase) = segmenter.finish() {
                    segments.extend(transcribe(phrase)?);
                }
                ensure!(!invalid.load(Ordering::SeqCst), "live ASR queue overflowed");
                ensure!(!cancelled.load(Ordering::SeqCst), "live ASR cancelled");
                return Ok(LiveTranscript {
                    segments,
                    audio_samples,
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("live ASR disconnected"))
            }
        }
    }
}

impl Drop for StreamingTranscription {
    fn drop(&mut self) {
        if !self.finished {
            self.cancel.store(true, Ordering::SeqCst);
        }
    }
}

pub fn transcribe_phrase(
    model: &ModelService,
    model_path: &std::path::Path,
    mut phrase: AudioPhrase,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<Segment>> {
    ensure!(!cancel.load(Ordering::SeqCst), "live ASR cancelled");
    let length = phrase.pcm.len();
    // Whisper's minimum input guard must not drop a short final syllable.
    phrase.pcm.resize(length.max(RATE / 10), 0.0);
    let vad = model.vad_path_for_pcm(phrase.pcm.len())?;
    let start = Instant::now();
    let segments = model.transcribe_pcm_with_progress(
        model_path,
        &phrase.pcm,
        vad.as_deref(),
        Some(cancel),
        "capture/live",
        |_| {},
        None,
    )?;
    let mut segments = filter_hallucination_phrases(&segments);
    let offset_ms = (phrase.start_sample * 1000 / RATE) as i64;
    let duration_ms = (length * 1000 / RATE) as i64;
    for segment in &mut segments {
        segment.start_ms = segment.start_ms.clamp(0, duration_ms) + offset_ms;
        segment.end_ms = segment
            .end_ms
            .clamp(0, duration_ms)
            .max(segment.start_ms - offset_ms)
            + offset_ms;
    }
    tracing::info!(
        start_ms = offset_ms,
        audio_ms = duration_ms,
        inference_ms = start.elapsed().as_millis() as u64,
        "capture phrase transcribed"
    );
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    impl AudioSegmenter for PhraseBuffer {
        fn push(&mut self, pcm: &[f32]) -> Result<Vec<AudioPhrase>> {
            self.push(pcm, detect)
        }
        fn finish(self) -> Option<AudioPhrase> {
            self.finish()
        }
    }

    #[test]
    fn audio_tap_transcribes_before_stop_then_flushes_tail_in_order() {
        let (done, received) = mpsc::channel();
        let live = StreamingTranscription::spawn(move |rx, cancel, failed| {
            run_worker(rx, cancel, failed, PhraseBuffer::default(), |phrase| {
                done.send(phrase.start_sample).unwrap();
                Ok(vec![Segment::new(
                    phrase.start_sample as i64,
                    phrase.ready_sample as i64,
                    "words",
                )])
            })
        })
        .unwrap();
        live.tap()(&fixture());
        assert_eq!(received.recv_timeout(Duration::from_secs(2)).unwrap(), 0);
        let cancel = live.cancel_flag();
        let result = live.finish().unwrap();
        assert!(
            !cancel.load(Ordering::SeqCst),
            "successful finish must not look cancelled to the controller"
        );
        assert_eq!(result.audio_samples, fixture().len());
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[1].start_ms, (STEP * 65) as i64);
    }

    #[test]
    fn record_worker_tail_and_completed_phrases_reach_final_speaker_alignment() {
        use crate::services::transcription::{
            run_post_capture_with_mic_segments, CaptureAudio, CaptureProfile, PostCaptureInput,
            SpeakerEvidenceInput,
        };
        use crate::types::DiarizationRange;
        let (done, received) = mpsc::channel();
        let live = StreamingTranscription::spawn(move |rx, cancel, failed| {
            run_worker(rx, cancel, failed, PhraseBuffer::default(), |phrase| {
                done.send(()).unwrap();
                Ok(vec![Segment::new(
                    (phrase.start_sample * 1000 / RATE) as i64,
                    (phrase.ready_sample * 1000 / RATE) as i64,
                    "recorded words",
                )])
            })
        })
        .unwrap();
        let abort = live.cancel_flag();
        live.tap()(&fixture());
        received.recv_timeout(Duration::from_secs(2)).unwrap(); // work before Stop
        let transcript = live.finish().unwrap();
        let split = (STEP * 65 * 1000 / RATE) as u64;
        let ranges = [
            DiarizationRange {
                speaker_id: 0,
                start_ms: 0,
                end_ms: split,
            },
            DiarizationRange {
                speaker_id: 1,
                start_ms: split,
                end_ms: 60_000,
            },
        ];
        let temp = tempfile::tempdir().unwrap();
        let model = ModelService::new(temp.path().to_path_buf());
        // No model on disk: successful assembly proves no hidden second ASR pass.
        let result = run_post_capture_with_mic_segments(
            &model,
            PostCaptureInput {
                profile: CaptureProfile::Record,
                audio: CaptureAudio {
                    mic_pcm_16k: &[],
                    speaker_pcm_16k: None,
                },
                model_path: &temp.path().join("absent.bin"),
                speaker_evidence: Some(SpeakerEvidenceInput::LiveRanges(&ranges)),
                abort: Some(abort),
                on_model_loaded: None,
            },
            Some(transcript.segments),
            |_| {},
        )
        .unwrap();
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].speaker.as_deref(), Some("Speaker 1"));
        assert_eq!(result.segments[1].speaker.as_deref(), Some("Speaker 2"));
        assert_eq!(result.segments[1].start_ms, split as i64);
    }

    #[test]
    fn cancelling_at_stop_discards_completed_phrases_and_tail() {
        let (done, received) = mpsc::channel();
        let live = StreamingTranscription::spawn(move |rx, cancel, failed| {
            run_worker(rx, cancel, failed, PhraseBuffer::default(), |_| {
                done.send(()).unwrap();
                Ok(vec![Segment::new(0, 1000, "must not save")])
            })
        })
        .unwrap();
        live.tap()(&fixture());
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        live.cancel_flag().store(true, Ordering::SeqCst);
        assert!(live.finish().is_err());
    }

    #[test]
    fn inference_failure_discards_partial_results_without_cancelling_fallback() {
        let live = StreamingTranscription::spawn(move |rx, cancel, failed| {
            run_worker(rx, cancel, failed, PhraseBuffer::default(), |_| {
                Err(anyhow!("test inference failure"))
            })
        })
        .unwrap();
        let cancel = live.cancel_flag();
        live.tap()(&fixture());
        assert!(live
            .finish()
            .unwrap_err()
            .to_string()
            .contains("test inference failure"));
        assert!(!cancel.load(Ordering::SeqCst));
    }

    #[test]
    fn dropping_capture_cancels_queued_work_without_waiting_for_inference() {
        let (entered, received) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let mut live = StreamingTranscription::spawn(move |rx, cancel, failed| {
            run_worker(rx, cancel, failed, PhraseBuffer::default(), |_| {
                entered.send(()).unwrap();
                blocked.recv().unwrap();
                Ok(Vec::new())
            })
        })
        .unwrap();
        let tap = live.tap();
        tap(&fixture());
        received.recv_timeout(Duration::from_secs(2)).unwrap();
        let worker = live.worker.take().unwrap();
        drop(live);
        release.send(()).unwrap();
        assert!(worker.join().unwrap().is_err());
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn overflowing_tap_invalidates_results_instead_of_losing_audio_silently() {
        let (release, blocked) = mpsc::channel();
        let live = StreamingTranscription::spawn(move |rx, cancel, failed| {
            blocked.recv().unwrap();
            run_worker(rx, cancel, failed, PhraseBuffer::default(), |_| {
                Ok(Vec::new())
            })
        })
        .unwrap();
        live.tap()(&vec![0.1; PACKET * 513]);
        assert!(live.failed.load(Ordering::SeqCst));
        release.send(()).unwrap();
        assert!(live.finish().is_err());
    }

    fn detect(pcm: &[f32]) -> Result<Vec<f32>> {
        Ok(pcm
            .chunks(512)
            .map(|frame| {
                if frame.iter().any(|v| *v != 0.0) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect())
    }

    fn fixture() -> Vec<f32> {
        let mut pcm = vec![0.1; STEP * 64];
        pcm.extend(vec![0.0; STEP * 2]);
        pcm.extend(vec![0.2; STEP * 5 + 123]);
        pcm
    }

    #[test]
    fn closes_phrase_before_stop_and_preserves_every_sample_once() {
        let pcm = fixture();
        let mut buffer = PhraseBuffer::default();
        let mut phrases = buffer.push(&pcm, detect).unwrap();
        assert_eq!(phrases.len(), 1, "must produce work before Stop");
        assert!(phrases[0].ready_sample < pcm.len());
        phrases.push(buffer.finish().unwrap());
        assert_eq!(phrases[1].start_sample, phrases[0].pcm.len());
        assert_eq!(
            phrases
                .iter()
                .flat_map(|p| p.pcm.iter())
                .copied()
                .collect::<Vec<_>>(),
            pcm
        );
        assert_eq!(phrases.last().unwrap().ready_sample, pcm.len());
    }

    #[test]
    fn callback_packet_size_does_not_change_phrase_boundaries() {
        let pcm = fixture();
        for packet in [1, 511, 1280, 16000] {
            let mut buffer = PhraseBuffer::default();
            let mut phrases = Vec::new();
            for part in pcm.chunks(packet) {
                phrases.extend(buffer.push(part, detect).unwrap());
            }
            phrases.push(buffer.finish().unwrap());
            assert_eq!(
                phrases.iter().map(|p| p.ready_sample).collect::<Vec<_>>(),
                [STEP * 65, pcm.len()]
            );
        }
    }

    #[test]
    fn short_pause_does_not_split_a_word_or_phrase() {
        let mut pcm = vec![0.1; STEP * 64];
        pcm.extend(vec![0.0; 512 * 8]);
        pcm.extend(vec![0.2; STEP]);
        let mut buffer = PhraseBuffer::default();
        assert!(buffer.push(&pcm, detect).unwrap().is_empty());
        assert_eq!(buffer.finish().unwrap().pcm, pcm);
    }

    #[test]
    fn continuous_speech_exceeding_limit_requests_full_wav_fallback() {
        let mut buffer = PhraseBuffer::default();
        assert!(buffer.push(&vec![0.1; MAX_PENDING + 1], detect).is_err());
    }

    #[test]
    fn short_final_audio_is_retained_and_empty_capture_has_no_tail() {
        assert!(PhraseBuffer::default().finish().is_none());
        let mut buffer = PhraseBuffer::default();
        buffer.push(&[0.1; 17], detect).unwrap();
        assert_eq!(buffer.finish().unwrap().pcm.len(), 17);
    }
}
