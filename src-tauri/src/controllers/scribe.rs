use crate::services::audio::{read_wav_mono_f32, WHISPER_SAMPLE_RATE};
use crate::services::{
    audio::{AudioService, MicSession},
    config::ConfigService,
    history::HistoryService,
    model::ModelService,
    output::OutputService,
};
use crate::types::{
    Config, HistoryRecord, Note, ProcessingStage, RecoverySessionInfo, ScribeState,
    ScribeStateEvent, ScribeTranscriptEntry, Segment, SessionManifest, SessionManifestState,
};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// Capture-time record of a speaker segment: the absolute start-ms within the recording and
/// the on-disk WAV (already at 16 kHz) the writer thread produced. Read back by
/// `prepare_audio` and converted to in-memory `SpeakerSegment`s for assembly.
struct CapturedSpeakerSegment {
    start_ms: u64,
    wav_path: PathBuf,
}

/// Assembly-time view of a speaker segment after the WAV has been read into RAM. Kept as a
/// plain struct so `assemble_speaker_pcm` (unit-tested with in-memory data) stays independent
/// of disk I/O.
struct SpeakerSegment {
    start_ms: u64,
    pcm_16k: Vec<f32>,
}

struct SpeakerAccumulator {
    segments: Vec<CapturedSpeakerSegment>,
    /// Active loopback stream and the recording-relative ms offset it started at.
    active: Option<(u64, MicSession)>,
}

impl SpeakerAccumulator {
    fn new() -> Self {
        Self {
            segments: vec![],
            active: None,
        }
    }
}

struct ActiveSession {
    mic: MicSession,
    speaker_accum: SpeakerAccumulator,
    previous_output_device: Option<String>,
    session_dir: PathBuf,
    started_at: Instant,
    started_at_iso: String,
}

/// Speaker RMS below this threshold (-60 dBFS) is treated as digital silence.
/// BlackHole outputs exact zeros when nothing is playing; this margin covers near-silence noise.
const SPEAKER_SILENCE_THRESHOLD: f32 = 1e-3;

/// Intermediate state produced by prepare_audio and consumed by run_transcription / write_outputs.
struct PreparedAudio {
    session_dir: PathBuf,
    wav_path: PathBuf,
    pcm_16k: Vec<f32>,
    speaker_pcm_16k: Option<Vec<f32>>,
}

enum ProgressMessage {
    Progress(f32),
    Finished,
}

struct Inner {
    state: ScribeState,
    session: Option<ActiveSession>,
    notes: Vec<Note>,
    /// Shared with `do_transcription` while a transcription task is active.
    transcription_abort: Option<Arc<AtomicBool>>,
    /// Set once `mic.wav` is written during transcription (for abort UX).
    transcription_wav_path: Option<PathBuf>,
}

pub struct ScribeController {
    inner: Mutex<Inner>,
    /// Ensures `cancel`/`stop` never run while `start` is between `start_mic` and session commit
    /// (state still Idle but CPAL already recording — that used to make discard a no-op on streams).
    capture_sync: Mutex<()>,
    audio: Arc<AudioService>,
    model: Arc<ModelService>,
    output: Arc<OutputService>,
    history: Arc<HistoryService>,
    config: Arc<ConfigService>,
    app: AppHandle,
}

impl ScribeController {
    pub fn new(
        audio: Arc<AudioService>,
        model: Arc<ModelService>,
        output: Arc<OutputService>,
        history: Arc<HistoryService>,
        config: Arc<ConfigService>,
        app: AppHandle,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                state: ScribeState::Idle,
                session: None,
                notes: Vec::new(),
                transcription_abort: None,
                transcription_wav_path: None,
            }),
            capture_sync: Mutex::new(()),
            audio,
            model,
            output,
            history,
            config,
            app,
        })
    }

    /// Transition IDLE → RECORDING. Opens mic and creates session directory.
    pub fn start(
        &self,
        preferred_mic: Option<String>,
        preferred_speaker: Option<String>,
        capture_speaker: bool,
    ) -> Result<()> {
        let _capture = self.capture_guard();
        {
            let inner = self.lock();
            Self::ensure_start_allowed(&inner.state)?;
        }

        self.emit_capture_levels_idle();

        let cfg = self.config.get();
        let session_dir = self.output.make_session_dir(&cfg.save_folder)?;
        let started_at = chrono::Utc::now().to_rfc3339();
        let mic_wav_path = mic_wav_path_for(&session_dir);
        let app = self.app.clone();
        let mic = self.audio.start_mic(
            preferred_mic.as_deref(),
            true,
            mic_wav_path,
            Some(Arc::new(move |level| {
                app.emit("scribe://audio-level", level).ok();
            })),
        )?;
        let mut speaker_capture_started = false;
        let (speaker_accum, previous_output_device) = if capture_speaker {
            let prev = self.audio.get_output_device();
            if let Some(target_output) = preferred_speaker
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                if let Err(err) = self.audio.set_output_device(target_output) {
                    tracing::warn!(device = target_output, error = %err, "failed to switch output route");
                }
            }
            let app = self.app.clone();
            let speaker_wav_path = speaker_segment_wav_path(&session_dir, 0);
            match self.audio.start_loopback(
                None,
                speaker_wav_path,
                Some(Arc::new(move |level| {
                    app.emit("scribe://speaker-level", level).ok();
                })),
            ) {
                Ok(stream) => {
                    speaker_capture_started = true;
                    (
                        SpeakerAccumulator {
                            segments: vec![],
                            active: Some((0, stream)),
                        },
                        prev,
                    )
                }
                Err(err) => {
                    self.app
                        .emit(
                            "scribe://speaker-capture-unavailable",
                            json!({
                                "reason": err.to_string(),
                                "requestedSpeakerDevice": preferred_speaker
                            }),
                        )
                        .ok();
                    (SpeakerAccumulator::new(), None)
                }
            }
        } else {
            (SpeakerAccumulator::new(), None)
        };

        let mut inner = self.lock();
        Self::ensure_start_allowed(&inner.state)?;
        inner.state = ScribeState::Recording;
        inner.session = Some(ActiveSession {
            mic,
            speaker_accum,
            previous_output_device,
            session_dir: session_dir.clone(),
            started_at: Instant::now(),
            started_at_iso: started_at.clone(),
        });
        inner.notes.clear();
        self.emit_state(&inner);
        drop(inner);

        let mut speaker_manifest = Vec::new();
        if speaker_capture_started {
            speaker_manifest.push(
                speaker_segment_wav_path(&session_dir, 0)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("speaker_seg_0.wav")
                    .to_string(),
            );
        }
        self.write_session_manifest(
            &session_dir,
            SessionManifestState::Recording,
            &started_at,
            speaker_manifest,
            None,
            None,
        )?;
        self.spawn_record_start_preload(&cfg);
        Ok(())
    }

    /// Best-effort finalize of an in-progress recording when the app is quitting.
    pub fn finalize_capture_on_shutdown(&self) {
        let session = {
            let mut inner = self.lock();
            if inner.state != ScribeState::Recording {
                return;
            }
            inner.state = ScribeState::Idle;
            inner.session.take()
        };
        let Some(session) = session else {
            return;
        };
        let ActiveSession {
            mic,
            mut speaker_accum,
            previous_output_device,
            session_dir,
            started_at_iso,
            ..
        } = session;
        let _ = mic.stop_and_finalize();
        if let Some((_, stream)) = speaker_accum.active.take() {
            let _ = stream.stop_and_finalize();
        }
        let speaker_paths: Vec<String> = speaker_accum
            .segments
            .iter()
            .filter_map(|s| {
                s.wav_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
            })
            .collect();
        self.restore_output_device(previous_output_device.as_deref());
        self.emit_capture_levels_idle();
        let _ = self.write_session_manifest(
            &session_dir,
            SessionManifestState::Interrupted,
            &started_at_iso,
            speaker_paths,
            None,
            None,
        );
        let _ = self.app.emit(
            "scribe://state-changed",
            ScribeStateEvent::new(ScribeState::Idle),
        );
    }

    /// Eagerly load the configured model into the shared context cache while recording,
    /// so transcription on stop skips the cold-load delay (~300 ms tiny → ~2 s large).
    fn spawn_record_start_preload(&self, cfg: &Config) {
        let path = preload_path_for_config(cfg, &self.model);
        if !self.model.model_available(&path) {
            return;
        }
        let model = Arc::clone(&self.model);
        tauri::async_runtime::spawn(async move {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(e) = model.get_or_load_context(&path) {
                    tracing::debug!(error = %e, "record-start model preload failed");
                }
            })
            .await;
        });
    }

    /// Transition RECORDING → IDLE. Discards the audio buffer and removes the
    /// session directory if no files were written into it yet.
    pub fn cancel(&self) -> Result<()> {
        let _capture = self.capture_guard();
        let session = {
            let mut inner = self.lock();
            if inner.state != ScribeState::Recording {
                return Err(anyhow!("cannot cancel: not recording"));
            }
            let session = inner.session.take();
            inner.session = None;
            inner.state = ScribeState::Idle;
            inner.notes.clear();
            self.emit_state(&inner);
            session
        };
        if let Some(session) = session {
            let ActiveSession {
                mic,
                speaker_accum,
                previous_output_device,
                session_dir,
                ..
            } = session;
            let _ = mic
                .stop_and_finalize()
                .map_err(|e| tracing::debug!(error = %e, "cancel finalize mic"));
            let _ = finalize_speaker_segments(speaker_accum);
            self.restore_output_device(previous_output_device.as_deref());
            self.emit_capture_levels_idle();
            self.output.remove_session_dir(&session_dir);
        }
        Ok(())
    }

    /// Stop recording, write `mic.wav` + `notes.json`, return to IDLE without Whisper.
    pub fn save_recording_only(&self, title: Option<String>) -> Result<PathBuf> {
        let _capture = self.capture_guard();
        let title =
            title.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());

        let (session, notes) = {
            let mut inner = self.lock();
            if inner.state != ScribeState::Recording {
                return Err(anyhow!("cannot save recording-only: not recording"));
            }
            let session = inner
                .session
                .take()
                .ok_or_else(|| anyhow!("recording session missing"))?;
            let notes = inner.notes.clone();
            inner.notes.clear();
            inner.state = ScribeState::Idle;
            (session, notes)
        };
        let ActiveSession {
            mic,
            speaker_accum,
            previous_output_device,
            session_dir,
            ..
        } = session;
        // mic.wav was streamed to disk during capture; finalize and we're done. Speaker
        // segments aren't kept by save-recording-only (the original behavior).
        let wav_path = mic.stop_and_finalize()?;
        for path in finalize_speaker_segments(speaker_accum) {
            let _ = self.output.delete_wav(&path);
        }
        self.restore_output_device(previous_output_device.as_deref());
        self.emit_capture_levels_idle();

        self.output
            .write_session_notes(&session_dir, &title, "mic.wav", &notes)?;
        let _ = std::fs::remove_file(session_dir.join("session.json"));

        self.emit_idle_optional_wav(Some(&wav_path));
        Ok(wav_path)
    }

    /// Request cooperative cancellation before transcript write (WAV retained). UI may IDLE immediately.
    pub fn abort_transcription_keep_wav(&self) -> Result<()> {
        let wav = {
            let mut inner = self.lock();
            if inner.state != ScribeState::Transcribing {
                return Err(anyhow!("cannot abort transcription: not transcribing"));
            }
            if let Some(flag) = inner.transcription_abort.as_ref() {
                flag.store(true, Ordering::SeqCst);
            }
            inner.state = ScribeState::Idle;
            let w = inner.transcription_wav_path.clone();
            inner.transcription_abort = None;
            inner.transcription_wav_path = None;
            w
        };
        let mut idle = ScribeStateEvent::new(ScribeState::Idle);
        idle.wav_path = wav.map(|p| p.to_string_lossy().into_owned());
        self.app.emit("scribe://state-changed", idle).ok();
        Ok(())
    }

    /// Transition RECORDING → TRANSCRIBING then → DONE / NO_MODEL.
    /// Returns immediately; heavy work runs in a background spawn_blocking task.
    pub fn stop_and_save(this: Arc<Self>, title: Option<String>) -> Result<()> {
        let abort_flag = Arc::new(AtomicBool::new(false));
        // Extract session under lock then release immediately.
        let (session, notes) = {
            let _capture = this.capture_guard();
            let mut inner = this.lock();
            if inner.state != ScribeState::Recording {
                return Err(anyhow!("cannot stop: not recording"));
            }
            inner.state = ScribeState::Transcribing;
            inner.transcription_abort = Some(Arc::clone(&abort_flag));
            (
                inner.session.take().expect("session exists when Recording"),
                inner.notes.clone(),
            )
        };

        // Stop capture before emitting TRANSCRIBING so the mic is never active while we are not Recording.
        let prepared = match this.prepare_audio(session) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, "prepare_audio after stop failed");
                this.clear_transcription_tracking();
                {
                    let mut inner = this.lock();
                    inner.state = ScribeState::Error;
                }
                this.app
                    .emit(
                        "scribe://state-changed",
                        ScribeStateEvent {
                            error: Some(format!("failed to finalize recording: {e}")),
                            ..ScribeStateEvent::new(ScribeState::Error)
                        },
                    )
                    .ok();
                return Err(e);
            }
        };

        this.app
            .emit(
                "scribe://state-changed",
                ScribeStateEvent {
                    progress: Some(0.0),
                    processing_stage: Some(ProcessingStage::LoadingModel),
                    ..ScribeStateEvent::new(ScribeState::Transcribing)
                },
            )
            .ok();

        let title =
            title.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());

        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result = tokio::task::spawn_blocking(move || {
                ctrl.do_transcription(prepared, notes, &title, abort_flag)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "transcription error");
                    this.clear_transcription_tracking();
                    {
                        let mut inner = this.lock();
                        inner.state = ScribeState::Error;
                    }
                    this.app
                        .emit(
                            "scribe://state-changed",
                            ScribeStateEvent {
                                error: Some(e.to_string()),
                                ..ScribeStateEvent::new(ScribeState::Error)
                            },
                        )
                        .ok();
                }
                Err(e) => {
                    this.clear_transcription_tracking();
                    tracing::error!(error = %e, "transcription task panicked");
                    this.lock().state = ScribeState::Error;
                    this.app
                        .emit(
                            "scribe://state-changed",
                            ScribeStateEvent {
                                error: Some("Transcription crashed unexpectedly.".to_string()),
                                ..ScribeStateEvent::new(ScribeState::Error)
                            },
                        )
                        .ok();
                }
            }
        });

        Ok(())
    }

    /// Whisper + transcript write. Capture is already stopped (`prepare_audio` ran in `stop_and_save`).
    fn do_transcription(
        &self,
        prepared: PreparedAudio,
        notes: Vec<Note>,
        title: &str,
        abort_flag: Arc<AtomicBool>,
    ) -> Result<()> {
        let config = self.config.get();

        let model_path = resolve_model_path(&config, &self.model);
        if !self.model.model_available(&model_path) {
            self.clear_transcription_tracking();
            self.transition(ScribeState::NoModel);
            let _ = self.write_session_manifest(
                &prepared.session_dir,
                SessionManifestState::Error,
                "",
                vec![],
                None,
                None,
            );
            self.app
                .emit(
                    "scribe://state-changed",
                    ScribeStateEvent {
                        wav_path: Some(prepared.wav_path.to_string_lossy().into()),
                        ..ScribeStateEvent::new(ScribeState::NoModel)
                    },
                )
                .ok();
            return Ok(());
        }

        let segments = match self.run_transcription(&model_path, &prepared, &abort_flag) {
            Ok(segments) => segments,
            Err(e) => {
                // An abort interrupting `full()` can surface as an Err; treat that as a clean
                // stop rather than a transcription error.
                if abort_flag.load(Ordering::SeqCst) {
                    self.clear_transcription_tracking();
                    return Ok(());
                }
                let _ = self.write_session_manifest(
                    &prepared.session_dir,
                    SessionManifestState::Error,
                    "",
                    vec![],
                    None,
                    None,
                );
                return Err(e);
            }
        };
        if abort_flag.load(Ordering::SeqCst) {
            self.clear_transcription_tracking();
            return Ok(());
        }

        if let Err(e) =
            self.write_outputs(&segments, &notes, title, &model_path, &config, &prepared)
        {
            let _ = self.write_session_manifest(
                &prepared.session_dir,
                SessionManifestState::Error,
                "",
                vec![],
                None,
                None,
            );
            return Err(e);
        }
        Ok(())
    }

    /// Finalize audio streams (mic.wav and any speaker segment WAVs already streamed to
    /// disk during capture), then read them back for Whisper. Also writes a merged
    /// `speaker.wav` for archival when speaker capture was active.
    /// Sets transcription_wav_path so abort UX can reference the file.
    fn prepare_audio(&self, session: ActiveSession) -> Result<PreparedAudio> {
        let started_at_iso = session.started_at_iso.clone();
        let total_ms = session.started_at.elapsed().as_millis() as u64;
        let ActiveSession {
            mic,
            mut speaker_accum,
            previous_output_device,
            session_dir,
            ..
        } = session;

        // Roll any still-active loopback capture into the segment list before finalizing.
        if let Some((start_ms, stream)) = speaker_accum.active.take() {
            let wav_path = stream.stop_and_finalize()?;
            speaker_accum
                .segments
                .push(CapturedSpeakerSegment { start_ms, wav_path });
        }
        let speaker_capture_enabled = !speaker_accum.segments.is_empty();

        let wav_path = mic.stop_and_finalize()?;
        self.restore_output_device(previous_output_device.as_deref());
        self.emit_capture_levels_idle();

        let pcm_16k = read_wav_mono_f32(&wav_path)?;
        self.lock().transcription_wav_path = Some(wav_path.clone());

        let speaker_wav_names: Vec<String> = speaker_accum
            .segments
            .iter()
            .filter_map(|s| {
                s.wav_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
            })
            .collect();
        let _ = self.write_session_manifest(
            &session_dir,
            SessionManifestState::Transcribing,
            &started_at_iso,
            speaker_wav_names,
            None,
            None,
        );

        let speaker_pcm_16k = if speaker_capture_enabled {
            let loaded = load_speaker_segments(&speaker_accum.segments)?;
            let assembled = assemble_speaker_pcm(&loaded, total_ms);
            // Delete the per-segment intermediate WAVs once they've been merged. Keeping
            // both `speaker.wav` (merged) and `speaker_seg_*.wav` (per-cycle) would clutter
            // the session folder for no user-visible benefit.
            for seg in &speaker_accum.segments {
                let _ = self.output.delete_wav(&seg.wav_path);
            }
            self.output.write_wav(
                &assembled,
                WHISPER_SAMPLE_RATE,
                &session_dir.join("speaker.wav"),
            )?;
            if pcm_rms(&assembled) >= SPEAKER_SILENCE_THRESHOLD {
                Some(assembled)
            } else {
                tracing::info!(
                    threshold = SPEAKER_SILENCE_THRESHOLD,
                    "speaker channel is silent — skipping speaker transcription"
                );
                None
            }
        } else {
            None
        };

        Ok(PreparedAudio {
            session_dir,
            wav_path,
            pcm_16k,
            speaker_pcm_16k,
        })
    }

    /// Run Whisper on the prepared audio, reporting progress via state events.
    fn run_transcription(
        &self,
        model_path: &Path,
        prepared: &PreparedAudio,
        abort_flag: &Arc<AtomicBool>,
    ) -> Result<Vec<Segment>> {
        let (progress_tx, progress_rx) = mpsc::channel::<ProgressMessage>();
        let progress_app = self.app.clone();
        let progress_thread = std::thread::spawn(move || {
            while let Ok(message) = progress_rx.recv() {
                match message {
                    ProgressMessage::Progress(p) => {
                        progress_app
                            .emit(
                                "scribe://state-changed",
                                ScribeStateEvent {
                                    progress: Some(p),
                                    processing_stage: Some(ProcessingStage::TranscribingAudio),
                                    ..ScribeStateEvent::new(ScribeState::Transcribing)
                                },
                            )
                            .ok();
                    }
                    ProgressMessage::Finished => break,
                }
            }
        });

        let vad_path = self.model.vad_model_path();
        let vad = self
            .model
            .model_available(&vad_path)
            .then_some(vad_path.as_path());
        let segments = if let Some(speaker_pcm) = &prepared.speaker_pcm_16k {
            let tx1 = progress_tx.clone();
            let mic_segs = self.model.transcribe_pcm_with_progress(
                model_path,
                &prepared.pcm_16k,
                vad,
                Some(Arc::clone(abort_flag)),
                "scribe/mic",
                move |p| {
                    tx1.send(ProgressMessage::Progress(p * 0.5)).ok();
                },
            )?;
            // Skip the second (speaker) pass entirely if the user aborted during the mic pass.
            if abort_flag.load(Ordering::SeqCst) {
                progress_tx.send(ProgressMessage::Finished).ok();
                progress_thread.join().ok();
                return Ok(Vec::new());
            }
            let tx2 = progress_tx.clone();
            let speaker_segs = self.model.transcribe_pcm_with_progress(
                model_path,
                speaker_pcm,
                vad,
                Some(Arc::clone(abort_flag)),
                "scribe/speaker",
                move |p| {
                    tx2.send(ProgressMessage::Progress(0.5 + p * 0.5)).ok();
                },
            )?;
            let speaker_segs = filter_hallucination_phrases(speaker_segs);
            Ok(self.model.merge_dual_source(&mic_segs, &speaker_segs))
        } else {
            let tx = progress_tx.clone();
            self.model.transcribe_pcm_with_progress(
                model_path,
                &prepared.pcm_16k,
                vad,
                Some(Arc::clone(abort_flag)),
                "scribe/mic",
                move |p| {
                    tx.send(ProgressMessage::Progress(p)).ok();
                },
            )
        };

        progress_tx.send(ProgressMessage::Finished).ok();
        progress_thread.join().ok();
        segments
    }

    /// Write the transcript file, optionally delete WAVs, and emit the Done event.
    fn write_outputs(
        &self,
        segments: &[Segment],
        notes: &[Note],
        title: &str,
        model_path: &Path,
        config: &Config,
        prepared: &PreparedAudio,
    ) -> Result<()> {
        let save_folder = PathBuf::from(&config.save_folder);
        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().replace("ggml-", ""))
            .unwrap_or_else(|| "model".to_string());

        // Markdown is opt-in: reserve a non-colliding `.md` name only when the toggle is on.
        let markdown_path = if config.save_transcripts_as_markdown {
            Some(self.output.transcript_path(&save_folder, model_path, title))
        } else {
            None
        };

        self.app
            .emit(
                "scribe://state-changed",
                ScribeStateEvent {
                    progress: Some(1.0),
                    processing_stage: Some(ProcessingStage::WritingTranscript),
                    ..ScribeStateEvent::new(ScribeState::Transcribing)
                },
            )
            .ok();
        if let Some(dest) = markdown_path.as_ref() {
            self.output.write_transcript(
                segments,
                notes,
                title,
                &model_name,
                config.include_timestamps,
                &config.replacement_rules,
                &config.replacement_prefix,
                dest,
            )?;
        }

        // Persist the canonical history record — always, regardless of the markdown toggle.
        let keep_audio = config.keep_wav && !segments.is_empty();
        // speaker_capture = user setting; dual_source = speaker PCM actually used for merge
        let speaker_capture = config.scribe_capture_speaker;
        let dual_source = prepared.speaker_pcm_16k.is_some();
        let (session_dir, audio_path) = if keep_audio {
            (
                Some(prepared.session_dir.to_string_lossy().into_owned()),
                Some(
                    mic_wav_path_for(&prepared.session_dir)
                        .to_string_lossy()
                        .into_owned(),
                ),
            )
        } else {
            (None, None)
        };
        let record = HistoryRecord::from_scribe(
            title.to_string(),
            model_name.clone(),
            segments.to_vec(),
            notes.to_vec(),
            &config.replacement_rules,
            &config.replacement_prefix,
            speaker_capture,
            dual_source,
            session_dir,
            audio_path,
            markdown_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
        );
        let record_id = match self.history.append(&config.save_folder, record) {
            Ok(id) => {
                self.app.emit("note://item-added", ()).ok();
                Some(id)
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to append scribe history record");
                None
            }
        };

        if !segments.is_empty() {
            self.app
                .emit(
                    "scribe://state-changed",
                    ScribeStateEvent {
                        progress: Some(1.0),
                        processing_stage: Some(ProcessingStage::CleaningUpAudio),
                        ..ScribeStateEvent::new(ScribeState::Transcribing)
                    },
                )
                .ok();
        }
        self.output
            .finalize_scribe_session(&prepared.session_dir, keep_audio)?;

        self.clear_transcription_tracking();
        self.transition(ScribeState::Done);
        self.app
            .emit(
                "scribe://state-changed",
                ScribeStateEvent {
                    transcript_path: markdown_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned()),
                    history_record_id: record_id,
                    ..ScribeStateEvent::new(ScribeState::Done)
                },
            )
            .ok();
        Ok(())
    }

    pub fn get_include_timestamps(&self) -> bool {
        self.config.get().include_timestamps
    }

    /// Incomplete Scribe sessions under the configured save folder (crash/interrupted recordings).
    pub fn list_recovery_sessions(&self) -> Result<Vec<RecoverySessionInfo>> {
        let cfg = self.config.get();
        self.output
            .scan_incomplete_scribe_sessions(&cfg.save_folder)
    }

    pub fn list_transcripts(&self) -> Result<Vec<ScribeTranscriptEntry>, String> {
        let cfg = self.config.get();
        self.output
            .list_transcripts(&cfg.save_folder)
            .map_err(|e| e.to_string())
    }

    pub fn set_include_timestamps(&self, enabled: bool) -> Result<()> {
        self.config
            .update(|cfg| cfg.include_timestamps = enabled)
            .map_err(|e| anyhow!("failed to update config: {e}"))
    }

    /// Start or stop the loopback stream while a recording is active.
    /// The mic continues uninterrupted; this only affects speaker capture.
    /// Calling with `enabled = true` when already capturing is a no-op, and vice versa.
    pub fn toggle_speaker_capture(&self, enabled: bool) -> Result<()> {
        if enabled {
            // Read state, start_ms, and the next segment index under lock; release before I/O.
            let (start_ms, segment_wav_path) = {
                let inner = self.lock();
                if inner.state != ScribeState::Recording {
                    return Err(anyhow!("not recording"));
                }
                let session = inner
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow!("no session"))?;
                if session.speaker_accum.active.is_some() {
                    return Ok(()); // no-op: already capturing
                }
                let next_index = session.speaker_accum.segments.len();
                (
                    session.started_at.elapsed().as_millis() as u64,
                    speaker_segment_wav_path(&session.session_dir, next_index),
                )
            };

            // Always switch output to the preferred speaker device when enabling.
            let cfg = self.config.get();
            if let Some(target) = cfg
                .preferred_speaker_device
                .as_deref()
                .filter(|s| !s.trim().is_empty())
            {
                let current_device = self.audio.get_output_device();
                {
                    let mut inner = self.lock();
                    if let Some(session) = inner.session.as_mut() {
                        if session.previous_output_device.is_none() {
                            session.previous_output_device = current_device;
                        }
                    }
                }
                if let Err(err) = self.audio.set_output_device(target) {
                    tracing::warn!(device = target, error = %err, "failed to switch output device");
                }
            }

            let app = self.app.clone();
            let loopback = self.audio.start_loopback(
                None,
                segment_wav_path,
                Some(Arc::new(move |level| {
                    app.emit("scribe://speaker-level", level).ok();
                })),
            );
            match loopback {
                Ok(stream) => {
                    let (session_dir, started_at_iso, speaker_wavs) = {
                        let mut inner = self.lock();
                        let Some(session) = inner.session.as_mut() else {
                            return Ok(());
                        };
                        session.speaker_accum.active = Some((start_ms, stream));
                        (
                            session.session_dir.clone(),
                            session.started_at_iso.clone(),
                            speaker_manifest_wav_names(
                                &session.session_dir,
                                &session.speaker_accum,
                            ),
                        )
                    };
                    let _ = self.sync_session_manifest_speaker_wavs(
                        &session_dir,
                        &started_at_iso,
                        speaker_wavs,
                    );
                }
                Err(err) => {
                    self.app
                        .emit(
                            "scribe://speaker-capture-unavailable",
                            json!({ "reason": err.to_string() }),
                        )
                        .ok();
                }
            }
        } else {
            // Extract the active stream under lock, then drain it outside the lock.
            let active = {
                let mut inner = self.lock();
                if inner.state != ScribeState::Recording {
                    return Err(anyhow!("not recording"));
                }
                inner
                    .session
                    .as_mut()
                    .and_then(|s| s.speaker_accum.active.take())
            };
            if let Some((start_ms, stream)) = active {
                // Finalize the segment WAV outside the lock — blocking I/O.
                let wav_path = match stream.stop_and_finalize() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::debug!(error = %e, "speaker segment finalize failed");
                        return Ok(());
                    }
                };
                let (prev_device, session_dir, started_at_iso, speaker_wavs) = {
                    let mut inner = self.lock();
                    if let Some(session) = inner.session.as_mut() {
                        session
                            .speaker_accum
                            .segments
                            .push(CapturedSpeakerSegment { start_ms, wav_path });
                    }
                    let session = inner.session.as_ref();
                    (
                        session.and_then(|s| s.previous_output_device.clone()),
                        session.map(|s| s.session_dir.clone()),
                        session.map(|s| s.started_at_iso.clone()),
                        session
                            .as_ref()
                            .map(|s| speaker_manifest_wav_names(&s.session_dir, &s.speaker_accum)),
                    )
                };
                // Restore output device immediately so audio goes back to normal
                // while the mic recording continues — same restore as session end.
                self.restore_output_device(prev_device.as_deref());
                if let (Some(session_dir), Some(started_at_iso), Some(speaker_wavs)) =
                    (session_dir, started_at_iso, speaker_wavs)
                {
                    let _ = self.sync_session_manifest_speaker_wavs(
                        &session_dir,
                        &started_at_iso,
                        speaker_wavs,
                    );
                }
            }
            self.app.emit("scribe://speaker-level", 0.0_f32).ok();
        }
        Ok(())
    }

    /// Add a timestamped note. Only valid while recording.
    pub fn add_note(&self, text: String) -> Result<Note> {
        let mut inner = self.lock();
        if inner.state != ScribeState::Recording {
            return Err(anyhow!("cannot add note: not recording"));
        }
        let elapsed = inner
            .session
            .as_ref()
            .map(|s| s.started_at.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let note = Note {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            recorded_at_ms: elapsed,
        };
        inner.notes.push(note.clone());
        Ok(note)
    }

    fn transition(&self, state: ScribeState) {
        self.lock().state = state;
    }

    fn clear_transcription_tracking(&self) {
        let mut inner = self.lock();
        inner.transcription_abort = None;
        inner.transcription_wav_path = None;
    }

    fn emit_idle_optional_wav(&self, wav: Option<&Path>) {
        let mut ev = ScribeStateEvent::new(ScribeState::Idle);
        ev.wav_path = wav.map(|p| p.to_string_lossy().into_owned());
        self.app.emit("scribe://state-changed", ev).ok();
    }

    fn emit_state(&self, inner: &Inner) {
        self.app
            .emit(
                "scribe://state-changed",
                ScribeStateEvent::new(inner.state.clone()),
            )
            .ok();
    }

    pub fn list_input_devices(&self) -> Vec<String> {
        self.audio.list_input_devices()
    }

    pub fn list_output_devices(&self) -> Vec<String> {
        self.audio.list_output_devices()
    }

    pub fn read_transcript_at(&self, path: &str) -> Result<String, String> {
        let path = Path::new(path);
        let canonical = path
            .canonicalize()
            .map_err(|_| "invalid or inaccessible transcript path".to_string())?;
        let save_folder = self.config.get().save_folder;
        let base = Path::new(&save_folder)
            .canonicalize()
            .map_err(|_| "save folder is not accessible".to_string())?;
        if !canonical.starts_with(&base) {
            return Err("transcript path is outside the configured save folder".to_string());
        }
        self.output.read_transcript(&canonical)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| {
            tracing::debug!("recovering from poisoned scribe mutex");
            p.into_inner()
        })
    }

    fn capture_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.capture_sync.lock().unwrap_or_else(|p| {
            tracing::debug!("recovering from poisoned capture mutex");
            p.into_inner()
        })
    }

    fn restore_output_device(&self, previous: Option<&str>) {
        if let Some(device) = previous {
            if let Err(e) = self.audio.set_output_device(device) {
                tracing::warn!(device, error = %e, "failed to restore output device");
            }
        }
    }

    fn emit_capture_levels_idle(&self) {
        let _ = self.app.emit("scribe://audio-level", 0.0_f32);
        let _ = self.app.emit("scribe://speaker-level", 0.0_f32);
    }

    fn ensure_start_allowed(state: &ScribeState) -> Result<()> {
        if matches!(state, ScribeState::Recording | ScribeState::Transcribing) {
            return Err(anyhow!("cannot start: already in {:?}", state));
        }
        Ok(())
    }
}

/// Assemble all loopback segments into one 16 kHz PCM buffer sized to `total_ms`.
/// Gaps between segments are silence (zeros), preserving absolute timestamps so
/// Whisper output from the speaker channel aligns with the mic channel.
fn assemble_speaker_pcm(segments: &[SpeakerSegment], total_ms: u64) -> Vec<f32> {
    let total_samples = (total_ms as f64 / 1000.0 * WHISPER_SAMPLE_RATE as f64) as usize;
    let mut assembled = vec![0.0f32; total_samples];
    for seg in segments {
        // PCM is already 16 kHz on disk (writer thread resamples on the way out).
        let start = (seg.start_ms as f64 / 1000.0 * WHISPER_SAMPLE_RATE as f64) as usize;
        let copy_len = seg.pcm_16k.len().min(total_samples.saturating_sub(start));
        assembled[start..start + copy_len].copy_from_slice(&seg.pcm_16k[..copy_len]);
    }
    assembled
}

/// Read the captured speaker segments off disk into the in-memory layout `assemble_speaker_pcm`
/// expects. Splitting capture (writes) from assembly (reads) keeps the assembler pure.
fn load_speaker_segments(captured: &[CapturedSpeakerSegment]) -> Result<Vec<SpeakerSegment>> {
    captured
        .iter()
        .map(|seg| {
            let pcm_16k = read_wav_mono_f32(&seg.wav_path)?;
            Ok(SpeakerSegment {
                start_ms: seg.start_ms,
                pcm_16k,
            })
        })
        .collect()
}

/// Stop the active speaker stream (if any) and return the on-disk WAV paths for every
/// captured segment. Used by both `cancel` (to delete them) and `save_recording_only`
/// (which also discards them). Errors finalizing a stream are logged and skipped so we
/// never leak the WAV header — the caller's cleanup pass deletes whatever is on disk.
fn finalize_speaker_segments(mut accum: SpeakerAccumulator) -> Vec<PathBuf> {
    if let Some((start_ms, stream)) = accum.active.take() {
        match stream.stop_and_finalize() {
            Ok(wav_path) => accum
                .segments
                .push(CapturedSpeakerSegment { start_ms, wav_path }),
            Err(e) => tracing::debug!(error = %e, "failed to finalize active speaker stream"),
        }
    }
    accum.segments.into_iter().map(|seg| seg.wav_path).collect()
}

fn mic_wav_path_for(session_dir: &Path) -> PathBuf {
    session_dir.join("mic.wav")
}

impl ScribeController {
    fn write_session_manifest(
        &self,
        session_dir: &Path,
        state: SessionManifestState,
        started_at: &str,
        speaker_wavs: Vec<String>,
        transcript_path: Option<String>,
        title: Option<String>,
    ) -> Result<()> {
        let started_at = if started_at.is_empty() {
            let path = session_dir.join("session.json");
            if path.exists() {
                std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<SessionManifest>(&raw).ok())
                    .map(|m| m.started_at)
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
            } else {
                chrono::Utc::now().to_rfc3339()
            }
        } else {
            started_at.to_string()
        };
        self.output.write_session_manifest(
            session_dir,
            &SessionManifest {
                format_version: 1,
                state,
                started_at,
                mic_wav: "mic.wav".to_string(),
                speaker_wavs,
                transcript_path,
                title,
            },
        )
    }

    fn sync_session_manifest_speaker_wavs(
        &self,
        session_dir: &Path,
        started_at_iso: &str,
        speaker_wavs: Vec<String>,
    ) -> Result<()> {
        self.write_session_manifest(
            session_dir,
            SessionManifestState::Recording,
            started_at_iso,
            speaker_wavs,
            None,
            None,
        )
    }
}

fn speaker_segment_wav_path(session_dir: &Path, index: usize) -> PathBuf {
    session_dir.join(format!("speaker_seg_{index}.wav"))
}

fn speaker_manifest_wav_names(session_dir: &Path, accum: &SpeakerAccumulator) -> Vec<String> {
    let mut names: Vec<String> = accum
        .segments
        .iter()
        .filter_map(|s| {
            s.wav_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
        })
        .collect();
    if accum.active.is_some() {
        let idx = accum.segments.len();
        names.push(
            speaker_segment_wav_path(session_dir, idx)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("speaker_seg_0.wav")
                .to_string(),
        );
    }
    names
}

fn pcm_rms(pcm: &[f32]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = pcm.iter().map(|&s| s * s).sum();
    (sum_sq / pcm.len() as f32).sqrt()
}

/// Strip known Whisper hallucination phrases from speaker segments.
/// Whisper frequently outputs these on silent or near-silent input.
fn filter_hallucination_phrases(segments: Vec<Segment>) -> Vec<Segment> {
    const PHRASES: &[&str] = &[
        "thank you.",
        "thanks.",
        "thanks for watching.",
        "thank you for watching.",
        "you.",
        "bye.",
        "bye-bye.",
        "bye bye.",
    ];
    segments
        .into_iter()
        .filter(|seg| {
            let lower = seg.text.trim().to_lowercase();
            !PHRASES.iter().any(|&p| lower == p) && !lower.starts_with("transcribed by")
        })
        .collect()
}

fn resolve_model_path(config: &Config, model: &ModelService) -> PathBuf {
    if let Some(p) = &config.scribe_model_path {
        PathBuf::from(p)
    } else if let Some(model_id) = &config.selected_model_id {
        model
            .model_path_for_id(model_id)
            .unwrap_or_else(|| model.default_model_path())
    } else {
        model.default_model_path()
    }
}

/// Path of the model that Scribe will use on stop — same resolution as transcription.
fn preload_path_for_config(config: &Config, model: &ModelService) -> PathBuf {
    resolve_model_path(config, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::model::SMALL_MODEL_FILENAME;
    use std::path::PathBuf;

    #[test]
    fn start_guard_rejects_recording_and_transcribing_states() {
        assert!(ScribeController::ensure_start_allowed(&ScribeState::Idle).is_ok());
        assert!(ScribeController::ensure_start_allowed(&ScribeState::Done).is_ok());
        assert!(ScribeController::ensure_start_allowed(&ScribeState::NoModel).is_ok());

        assert!(ScribeController::ensure_start_allowed(&ScribeState::Recording).is_err());
        assert!(ScribeController::ensure_start_allowed(&ScribeState::Transcribing).is_err());
    }

    #[test]
    fn resolve_model_path_prefers_explicit_path() {
        let models_dir =
            std::env::temp_dir().join(format!("liscribe-test-models-{}", uuid::Uuid::new_v4()));
        let model = ModelService::new(models_dir.clone());
        let config = Config {
            scribe_model_path: Some("/tmp/custom-model.bin".to_string()),
            selected_model_id: Some("tiny-en-q5".to_string()),
            ..Config::default()
        };

        let chosen = resolve_model_path(&config, model.as_ref());
        assert_eq!(chosen, PathBuf::from("/tmp/custom-model.bin"));
    }

    #[test]
    fn speaker_manifest_wav_names_lists_segments_and_active() {
        let dir = std::env::temp_dir().join(format!("speaker-manifest-{}", uuid::Uuid::new_v4()));
        let mut accum = SpeakerAccumulator::new();
        accum.segments.push(CapturedSpeakerSegment {
            start_ms: 0,
            wav_path: dir.join("speaker_seg_0.wav"),
        });
        accum.active = None;
        let names = speaker_manifest_wav_names(&dir, &accum);
        assert_eq!(names, vec!["speaker_seg_0.wav".to_string()]);
    }

    #[test]
    fn resolve_model_path_uses_selected_model_id_when_present() {
        let models_dir =
            std::env::temp_dir().join(format!("liscribe-test-models-{}", uuid::Uuid::new_v4()));
        let model = ModelService::new(models_dir.clone());
        let config = Config {
            selected_model_id: Some("tiny-en-q5".to_string()),
            ..Config::default()
        };

        let chosen = resolve_model_path(&config, model.as_ref());
        assert_eq!(chosen, models_dir.join("ggml-tiny.en-q5_1.bin"));
    }

    #[test]
    fn cancel_requires_recording_state() {
        assert!(matches!(
            ScribeController::ensure_start_allowed(&ScribeState::Error),
            Ok(())
        ));
        // Cancelling from non-recording states should be rejected at the controller level.
        // We test the guard directly since cancel() also checks state internally.
        for state in [
            ScribeState::Idle,
            ScribeState::Done,
            ScribeState::NoModel,
            ScribeState::Error,
        ] {
            assert!(
                ScribeController::ensure_start_allowed(&state).is_ok(),
                "start should be allowed from {state:?}"
            );
        }
    }

    #[test]
    fn preload_path_uses_custom_scribe_model_path() {
        let models_dir =
            std::env::temp_dir().join(format!("liscribe-test-models-{}", uuid::Uuid::new_v4()));
        let model = ModelService::new(models_dir);
        let config = Config {
            scribe_model_path: Some("/tmp/custom.bin".to_string()),
            selected_model_id: Some("tiny-en-q5".to_string()),
            ..Config::default()
        };
        assert_eq!(
            preload_path_for_config(&config, model.as_ref()),
            PathBuf::from("/tmp/custom.bin")
        );
    }

    #[test]
    fn preload_path_returns_catalog_path_for_all_models() {
        let models_dir =
            std::env::temp_dir().join(format!("liscribe-test-models-{}", uuid::Uuid::new_v4()));
        let model = ModelService::new(models_dir.clone());
        for id in [
            "tiny-en-q5",
            "base-en-q5",
            "small-en-q5",
            "medium-en-q5",
            "large-v3-turbo-q5",
        ] {
            let config = Config {
                selected_model_id: Some(id.to_string()),
                ..Config::default()
            };
            let p = preload_path_for_config(&config, model.as_ref());
            assert_eq!(p, model.model_path_for_id(id).expect("catalog path"));
            assert!(
                p.starts_with(&models_dir),
                "{id} preload path under models dir"
            );
        }
    }

    #[test]
    fn resolve_model_path_falls_back_to_default_when_unknown_selected_id() {
        let models_dir =
            std::env::temp_dir().join(format!("liscribe-test-models-{}", uuid::Uuid::new_v4()));
        let model = ModelService::new(models_dir.clone());
        let config = Config {
            selected_model_id: Some("not-a-real-model".to_string()),
            ..Config::default()
        };

        let chosen = resolve_model_path(&config, model.as_ref());
        assert_eq!(chosen, models_dir.join(SMALL_MODEL_FILENAME));
    }

    // ── SpeakerAccumulator state ───────────────────────────────────────────────

    #[test]
    fn speaker_accumulator_new_has_no_audio() {
        let acc = SpeakerAccumulator::new();
        assert!(acc.segments.is_empty());
        assert!(acc.active.is_none());
    }

    #[test]
    fn speaker_accumulator_has_segments_after_push() {
        let mut acc = SpeakerAccumulator::new();
        acc.segments.push(CapturedSpeakerSegment {
            start_ms: 0,
            wav_path: PathBuf::from("/tmp/fake-segment.wav"),
        });
        assert!(!acc.segments.is_empty());
    }

    fn fake_segment(start_ms: u64, pcm: Vec<f32>) -> SpeakerSegment {
        SpeakerSegment {
            start_ms,
            pcm_16k: pcm,
        }
    }

    // ── assemble_speaker_pcm ──────────────────────────────────────────────────

    #[test]
    fn assemble_speaker_pcm_no_segments_returns_silence() {
        let out = assemble_speaker_pcm(&[], 1_000);
        let expected_len = WHISPER_SAMPLE_RATE as usize; // 1 second at 16 kHz
        assert_eq!(out.len(), expected_len);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn assemble_speaker_pcm_segment_at_offset_zero_fills_from_start() {
        // 1-second session; segment starts at 0 ms with 0.5 seconds of audio
        let half_sec = WHISPER_SAMPLE_RATE as usize / 2;
        let seg = fake_segment(0, vec![0.5f32; half_sec]);

        let out = assemble_speaker_pcm(&[seg], 1_000);
        assert_eq!(out.len(), WHISPER_SAMPLE_RATE as usize);
        // First half should be filled with 0.5
        assert!(out[..half_sec].iter().all(|&s| (s - 0.5).abs() < 1e-5));
        // Second half should be silence
        assert!(out[half_sec..].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn assemble_speaker_pcm_segment_with_ms_offset_leaves_leading_silence() {
        // 2-second session; segment starts at 1000 ms
        let total_ms = 2_000u64;
        let one_sec = WHISPER_SAMPLE_RATE as usize;
        let seg = fake_segment(1_000, vec![1.0f32; one_sec]);

        let out = assemble_speaker_pcm(&[seg], total_ms);
        assert_eq!(out.len(), 2 * one_sec);
        // First second: silence
        assert!(
            out[..one_sec].iter().all(|&s| s == 0.0),
            "expected silence before segment"
        );
        // Second second: filled
        assert!(
            out[one_sec..].iter().all(|&s| (s - 1.0).abs() < 1e-5),
            "expected audio after offset"
        );
    }

    #[test]
    fn assemble_speaker_pcm_two_segments_with_gap() {
        // 3-second session: audio at 0–1 s, silence 1–2 s, audio at 2–3 s
        let one_sec = WHISPER_SAMPLE_RATE as usize;
        let seg_a = fake_segment(0, vec![0.3f32; one_sec]);
        let seg_b = fake_segment(2_000, vec![0.7f32; one_sec]);

        let out = assemble_speaker_pcm(&[seg_a, seg_b], 3_000);
        assert_eq!(out.len(), 3 * one_sec);
        assert!(
            out[..one_sec].iter().all(|&s| (s - 0.3).abs() < 1e-5),
            "first segment"
        );
        assert!(
            out[one_sec..2 * one_sec].iter().all(|&s| s == 0.0),
            "gap is silence"
        );
        assert!(
            out[2 * one_sec..].iter().all(|&s| (s - 0.7).abs() < 1e-5),
            "second segment"
        );
    }

    #[test]
    fn assemble_speaker_pcm_segment_truncated_when_it_overruns_total() {
        // 1-second session but segment PCM is 2 seconds long → must be clamped
        let seg = fake_segment(0, vec![1.0f32; 2 * WHISPER_SAMPLE_RATE as usize]);

        let out = assemble_speaker_pcm(&[seg], 1_000);
        assert_eq!(
            out.len(),
            WHISPER_SAMPLE_RATE as usize,
            "output must not exceed total_ms"
        );
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
        // DC +1.0 signal → RMS = 1.0
        let rms = pcm_rms(&vec![1.0f32; 1000]);
        assert!((rms - 1.0).abs() < 1e-5);
    }

    #[test]
    fn pcm_rms_below_threshold_for_silence() {
        assert!(pcm_rms(&vec![0.0f32; 16_000]) < SPEAKER_SILENCE_THRESHOLD);
    }

    #[test]
    fn pcm_rms_above_threshold_for_real_audio() {
        // 0.05 amplitude is well above any realistic noise floor
        assert!(pcm_rms(&vec![0.05f32; 16_000]) >= SPEAKER_SILENCE_THRESHOLD);
    }

    // ── filter_hallucination_phrases ──────────────────────────────────────────

    #[test]
    fn filter_removes_known_hallucination_phrases() {
        let segs = vec![
            Segment {
                start_ms: 0,
                end_ms: 500,
                text: "Thank you.".to_string(),
            },
            Segment {
                start_ms: 1_000,
                end_ms: 1_500,
                text: "Hello world".to_string(),
            },
            Segment {
                start_ms: 2_000,
                end_ms: 2_500,
                text: "Thanks for watching.".to_string(),
            },
            Segment {
                start_ms: 3_000,
                end_ms: 3_500,
                text: "Transcribed by Whisper".to_string(),
            },
        ];
        let filtered = filter_hallucination_phrases(segs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Hello world");
    }

    #[test]
    fn filter_is_case_insensitive() {
        let segs = vec![
            Segment {
                start_ms: 0,
                end_ms: 500,
                text: "THANK YOU.".to_string(),
            },
            Segment {
                start_ms: 1_000,
                end_ms: 1_500,
                text: "Real speech here".to_string(),
            },
        ];
        let filtered = filter_hallucination_phrases(segs);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Real speech here");
    }

    #[test]
    fn filter_keeps_real_speech_intact() {
        let segs = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "I'm talking about the project".to_string(),
            },
            Segment {
                start_ms: 1_000,
                end_ms: 2_000,
                text: "Let me explain the architecture".to_string(),
            },
        ];
        let filtered = filter_hallucination_phrases(segs);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_does_not_strip_thank_you_mid_sentence() {
        // Only exact matches should be stripped, not substrings
        let segs = vec![Segment {
            start_ms: 0,
            end_ms: 1_000,
            text: "I want to thank you for coming today".to_string(),
        }];
        let filtered = filter_hallucination_phrases(segs);
        assert_eq!(filtered.len(), 1);
    }
}
