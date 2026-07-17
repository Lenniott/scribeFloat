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
use anyhow::{anyhow, Context, Result};
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

/// Incremental diarization capability driving the live Record worker.
/// Implementations buffer internally and may return zero ranges per feed.
pub trait StreamingDiarizer: Send {
    fn feed(&mut self, pcm_16k: &[f32]) -> Result<Vec<DiarizationRange>>;
    fn flush(&mut self) -> Result<Vec<DiarizationRange>>;
}

/// Drain the PCM channel into the diarizer until every sender is dropped, then
/// flush. First inference error aborts the loop — the recording itself is
/// never affected, the caller just gets no speaker evidence.
fn run_worker_loop(
    diarizer: &mut dyn StreamingDiarizer,
    rx: std::sync::mpsc::Receiver<Vec<f32>>,
) -> Result<Vec<DiarizationRange>> {
    let mut ranges = Vec::new();
    while let Ok(pcm) = rx.recv() {
        ranges.extend(diarizer.feed(&pcm)?);
    }
    ranges.extend(diarizer.flush()?);
    Ok(ranges)
}

/// A live diarization worker attached to one recording session. PCM flows in
/// through [`Self::tap`]; results come back from [`Self::finish`].
pub struct LiveDiarization {
    tx: std::sync::mpsc::Sender<Vec<f32>>,
    handle: std::thread::JoinHandle<Result<Vec<DiarizationRange>>>,
}

impl LiveDiarization {
    fn spawn<D, F>(make_diarizer: F) -> Self
    where
        D: StreamingDiarizer + 'static,
        F: FnOnce() -> Result<D> + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
        let handle = std::thread::Builder::new()
            .name("live-diarization".into())
            .spawn(move || {
                // Model load happens here, not on the caller: early PCM just
                // buffers in the channel until the diarizer is ready.
                let mut diarizer = make_diarizer()?;
                run_worker_loop(&mut diarizer, rx)
            })
            .expect("spawn live-diarization thread");
        Self { tx, handle }
    }

    /// Cheap PCM forwarder for the audio writer thread's 16 kHz tap. Send
    /// errors are ignored: a dead worker must never disturb WAV writes.
    pub fn tap(&self) -> crate::services::audio::Pcm16kTap {
        let tx = self.tx.clone();
        Arc::new(move |pcm_16k: &[f32]| {
            let _ = tx.send(pcm_16k.to_vec());
        })
    }

    /// Close the channel, wait for the final flush, and return the ranges.
    /// `None` on any worker failure (degrade to a plain transcript).
    ///
    /// Callers must drop every tap clone first — for Record that means calling
    /// this only after `MicSession::stop_and_finalize()` joins the writer
    /// thread — or the worker never sees the channel close and this deadlocks.
    pub fn finish(self) -> Option<Vec<DiarizationRange>> {
        drop(self.tx);
        match self.handle.join() {
            Ok(Ok(ranges)) => Some(ranges),
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "live diarization failed — saving plain transcript");
                None
            }
            Err(_) => {
                tracing::warn!("live diarization worker panicked — saving plain transcript");
                None
            }
        }
    }

    /// Discard the session (recording cancelled). Joins the worker so its
    /// ONNX state is released promptly.
    pub fn cancel(self) {
        drop(self.tx);
        let _ = self.handle.join();
    }
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

    /// Spawn the live worker for one recording, or `None` when the model is
    /// missing (recording proceeds without speaker labels). The ~2 s model
    /// load happens inside the worker thread while early PCM buffers in the
    /// channel, so capture start is never delayed.
    pub fn start_live_session(self: &Arc<Self>) -> Option<LiveDiarization> {
        if !self.model_available() {
            tracing::info!("diarization model missing — recording without speaker labels");
            return None;
        }
        let service = Arc::clone(self);
        Some(LiveDiarization::spawn(move || {
            Ok(SortformerStreaming {
                inner: service.load_sortformer()?,
            })
        }))
    }

    fn load_sortformer(&self) -> Result<parakeet_rs::sortformer::Sortformer> {
        anyhow::ensure!(
            self.model_available(),
            "diarization model missing: {}",
            self.model_path.display()
        );
        // DIHARD3 tuning (min_duration_on ~7ms) over CallHome's (~0.5s): our
        // recordings are short, casual conversations, not clean phone calls,
        // so a brief third/fourth speaker needs to register as a distinct
        // slot instead of being absorbed into a longer-talking speaker.
        parakeet_rs::sortformer::Sortformer::with_config(
            &self.model_path,
            None,
            parakeet_rs::sortformer::DiarizationConfig::dihard3(),
        )
        .map_err(|e| anyhow::anyhow!("failed to load diarization model: {e}"))
    }
}

/// Adapts the parakeet-rs Sortformer streaming API to [`StreamingDiarizer`],
/// converting sample offsets (absolute across feeds) to ms.
struct SortformerStreaming {
    inner: parakeet_rs::sortformer::Sortformer,
}

impl StreamingDiarizer for SortformerStreaming {
    fn feed(&mut self, pcm_16k: &[f32]) -> Result<Vec<DiarizationRange>> {
        let segments = self
            .inner
            .feed(pcm_16k)
            .map_err(|e| anyhow!("diarization feed failed: {e}"))?;
        Ok(segments
            .iter()
            .map(|s| range_from_samples(s.speaker_id, s.start, s.end))
            .collect())
    }

    fn flush(&mut self) -> Result<Vec<DiarizationRange>> {
        let segments = self
            .inner
            .flush()
            .map_err(|e| anyhow!("diarization flush failed: {e}"))?;
        Ok(segments
            .iter()
            .map(|s| range_from_samples(s.speaker_id, s.start, s.end))
            .collect())
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

    /// Scripted streaming diarizer: one canned response per feed, then a
    /// canned flush. `Err` entries simulate mid-stream inference failure.
    struct FakeStreaming {
        feeds: std::sync::Mutex<std::collections::VecDeque<Result<Vec<DiarizationRange>>>>,
        flush: std::sync::Mutex<Option<Result<Vec<DiarizationRange>>>>,
        flushed: Arc<std::sync::atomic::AtomicBool>,
    }

    impl FakeStreaming {
        fn new(
            feeds: Vec<Result<Vec<DiarizationRange>>>,
            flush: Result<Vec<DiarizationRange>>,
        ) -> (Self, Arc<std::sync::atomic::AtomicBool>) {
            let flushed = Arc::new(std::sync::atomic::AtomicBool::new(false));
            (
                Self {
                    feeds: std::sync::Mutex::new(feeds.into()),
                    flush: std::sync::Mutex::new(Some(flush)),
                    flushed: Arc::clone(&flushed),
                },
                flushed,
            )
        }
    }

    impl StreamingDiarizer for FakeStreaming {
        fn feed(&mut self, _pcm_16k: &[f32]) -> Result<Vec<DiarizationRange>> {
            self.feeds
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(Vec::new()))
        }

        fn flush(&mut self) -> Result<Vec<DiarizationRange>> {
            self.flushed.store(true, std::sync::atomic::Ordering::SeqCst);
            self.flush.lock().unwrap().take().unwrap_or(Ok(Vec::new()))
        }
    }

    fn r(speaker_id: u8, start_ms: u64, end_ms: u64) -> DiarizationRange {
        DiarizationRange {
            speaker_id,
            start_ms,
            end_ms,
        }
    }

    #[test]
    fn worker_loop_accumulates_feeds_then_flush() {
        let (mut fake, _) = FakeStreaming::new(
            vec![Ok(vec![r(0, 0, 900)]), Ok(vec![]), Ok(vec![r(1, 900, 2_000)])],
            Ok(vec![r(0, 2_000, 2_500)]),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        for _ in 0..3 {
            tx.send(vec![0.0f32; 160]).unwrap();
        }
        drop(tx);
        let ranges = run_worker_loop(&mut fake, rx).unwrap();
        assert_eq!(ranges, vec![r(0, 0, 900), r(1, 900, 2_000), r(0, 2_000, 2_500)]);
    }

    #[test]
    fn worker_loop_stops_at_first_feed_error_without_flushing() {
        let (mut fake, flushed) = FakeStreaming::new(
            vec![Ok(vec![r(0, 0, 900)]), Err(anyhow!("onnx died"))],
            Ok(vec![]),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        for _ in 0..3 {
            tx.send(vec![0.0f32; 160]).unwrap();
        }
        drop(tx);
        assert!(run_worker_loop(&mut fake, rx).is_err());
        assert!(!flushed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn worker_loop_flush_only_when_no_pcm_arrives() {
        let (mut fake, flushed) = FakeStreaming::new(vec![], Ok(vec![r(2, 0, 100)]));
        let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
        drop(tx);
        let ranges = run_worker_loop(&mut fake, rx).unwrap();
        assert_eq!(ranges, vec![r(2, 0, 100)]);
        assert!(flushed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn live_session_tap_feeds_worker_and_finish_returns_ranges() {
        let live = LiveDiarization::spawn(|| {
            Ok(FakeStreaming::new(
                vec![Ok(vec![r(0, 0, 500)])],
                Ok(vec![r(1, 500, 900)]),
            )
            .0)
        });
        let tap = live.tap();
        tap(&[0.0f32; 160]);
        drop(tap);
        let ranges = live.finish().expect("worker result");
        assert_eq!(ranges, vec![r(0, 0, 500), r(1, 500, 900)]);
    }

    #[test]
    fn live_session_finish_is_none_when_worker_errors() {
        let live = LiveDiarization::spawn(|| {
            Ok(FakeStreaming::new(vec![Err(anyhow!("onnx died"))], Ok(vec![])).0)
        });
        let tap = live.tap();
        tap(&[0.0f32; 160]);
        drop(tap);
        assert!(live.finish().is_none());
    }

    #[test]
    fn live_session_finish_is_none_when_model_load_fails() {
        let live =
            LiveDiarization::spawn(|| -> Result<FakeStreaming> { Err(anyhow!("no model")) });
        // PCM sent while the loader is failing must not panic or block.
        let tap = live.tap();
        tap(&[0.0f32; 160]);
        drop(tap);
        assert!(live.finish().is_none());
    }

    #[test]
    fn live_session_cancel_joins_quietly() {
        let live = LiveDiarization::spawn(|| Ok(FakeStreaming::new(vec![], Ok(vec![])).0));
        let tap = live.tap();
        tap(&[0.0f32; 160]);
        drop(tap);
        live.cancel();
    }

    #[test]
    fn start_live_session_none_when_model_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(service_in(dir.path()).start_live_session().is_none());
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
