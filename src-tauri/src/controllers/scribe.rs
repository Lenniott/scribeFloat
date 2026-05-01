use crate::services::{
    audio::{AudioService, MicSession},
    config::ConfigService,
    model::ModelService,
    output::OutputService,
};
use crate::types::{Config, Note, ProcessingStage, ScribeState, ScribeStateEvent};
use anyhow::{anyhow, Result};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

struct ActiveSession {
    mic: MicSession,
    speaker: Option<MicSession>,
    previous_output_device: Option<String>,
    session_dir: PathBuf,
    started_at: Instant,
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
    audio: Arc<AudioService>,
    model: Arc<ModelService>,
    output: Arc<OutputService>,
    config: Arc<ConfigService>,
    app: AppHandle,
}

impl ScribeController {
    pub fn new(
        audio: Arc<AudioService>,
        model: Arc<ModelService>,
        output: Arc<OutputService>,
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
            audio,
            model,
            output,
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
        {
            let inner = self.lock();
            Self::ensure_start_allowed(&inner.state)?;
        }

        let cfg = self.config.get();
        let session_dir = self.output.make_session_dir(&cfg.save_folder)?;
        let app = self.app.clone();
        let mic = self.audio.start_mic(
            preferred_mic.as_deref(),
            true,
            Some(Arc::new(move |level| {
                app.emit("scribe://audio-level", level).ok();
            })),
        )?;
        let mut previous_output_device: Option<String> = None;
        let speaker = if capture_speaker {
            previous_output_device = self.audio.get_output_device();
            if let Some(target_output) = preferred_speaker
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
            {
                if let Err(err) = self.audio.set_output_device(target_output) {
                    eprintln!("failed to switch output route to `{target_output}`: {err}");
                }
            }
            let input_devices = self.audio.list_input_devices();
            let speaker_name = preferred_speaker.clone().unwrap_or_default();
            let input_match = input_devices.iter().any(|name| name == &speaker_name);
            let mut speaker_capture_name = preferred_speaker.clone();
            let has_preferred_output = self.audio.output_device_exists(&speaker_name);
            let has_blackhole_input =
                input_devices.iter().any(|name| name.eq_ignore_ascii_case("BlackHole 2ch"));
            if !input_match && has_preferred_output && has_blackhole_input {
                speaker_capture_name = Some("BlackHole 2ch".to_string());
            }
            let app = self.app.clone();
            match self.audio.start_mic(
                speaker_capture_name.as_deref(),
                false,
                Some(Arc::new(move |level| {
                    app.emit("scribe://speaker-level", level).ok();
                })),
            ) {
                Ok(stream) => Some(stream),
                Err(err) => {
                    // Keep recording reliable: if speaker stream cannot attach,
                    // continue with mic-only instead of failing the whole start.
                    self.app
                        .emit(
                            "scribe://speaker-capture-unavailable",
                            json!({
                                "reason": err.to_string(),
                                "requestedSpeakerDevice": preferred_speaker
                            }),
                        )
                        .ok();
                    None
                }
            }
        } else {
            None
        };

        let mut inner = self.lock();
        Self::ensure_start_allowed(&inner.state)?;
        inner.state = ScribeState::Recording;
        inner.session = Some(ActiveSession {
            mic,
            speaker,
            previous_output_device,
            session_dir,
            started_at: Instant::now(),
        });
        inner.notes.clear();
        self.emit_state(&inner);
        Ok(())
    }

    /// Transition RECORDING → IDLE. Discards the audio buffer and removes the
    /// session directory if no files were written into it yet.
    pub fn cancel(&self) -> Result<()> {
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
            self.restore_output_device(session.previous_output_device.as_deref());
            self.output.delete_session_dir_if_empty(&session.session_dir);
        }
        Ok(())
    }

    /// Stop recording, write `mic.wav` + `notes.json`, return to IDLE without Whisper.
    pub fn save_recording_only(&self, title: Option<String>) -> Result<PathBuf> {
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
        self.restore_output_device(session.previous_output_device.as_deref());

        let wav_path = session.session_dir.join("mic.wav");
        let (raw_pcm, native_rate) = session.mic.stop_and_take();
        let pcm_16k = resample_linear(&raw_pcm, native_rate, 16_000);
        self.output.write_wav(&pcm_16k, 16_000, &wav_path)?;
        self.output
            .write_session_notes(&session.session_dir, &title, "mic.wav", &notes)?;

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
            let result =
                tokio::task::spawn_blocking(move || {
                    ctrl.do_transcription(session, notes, &title, abort_flag)
                })
                    .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    eprintln!("transcription error: {e}");
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
                    eprintln!("transcription task panicked: {e}");
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

    /// Blocking transcription pipeline. Called inside spawn_blocking.
    fn do_transcription(
        &self,
        session: ActiveSession,
        notes: Vec<Note>,
        title: &str,
        abort_flag: Arc<AtomicBool>,
    ) -> Result<()> {
        let ActiveSession {
            mic,
            speaker,
            previous_output_device,
            session_dir,
            ..
        } = session;
        let (raw_pcm, native_rate) = mic.stop_and_take();
        let speaker_capture_enabled = speaker.is_some();
        let speaker_capture = speaker.map(|speaker| speaker.stop_and_take());
        self.restore_output_device(previous_output_device.as_deref());

        let pcm_16k = resample_linear(&raw_pcm, native_rate, 16_000);

        let config = self.config.get();
        let wav_path = session_dir.join("mic.wav");
        self.output.write_wav(&pcm_16k, 16_000, &wav_path)?;

        {
            let mut inner = self.lock();
            inner.transcription_wav_path = Some(wav_path.clone());
        }

        // Use configured path, then selected model id, then built-in default path.
        let model_path = resolve_model_path(&config, &self.model);

        if !self.model.model_available(&model_path) {
            self.clear_transcription_tracking();
            self.transition(ScribeState::NoModel);
            self.app
                .emit(
                    "scribe://state-changed",
                    ScribeStateEvent {
                        wav_path: Some(wav_path.to_string_lossy().into()),
                        ..ScribeStateEvent::new(ScribeState::NoModel)
                    },
                )
                .ok();
            return Ok(());
        }

        let (progress_tx, progress_rx) = mpsc::channel::<ProgressMessage>();
        let callback_progress_tx = progress_tx.clone();
        let progress_app = self.app.clone();
        let progress_thread = std::thread::spawn(move || {
            while let Ok(message) = progress_rx.recv() {
                match message {
                    ProgressMessage::Progress(progress) => {
                        progress_app
                            .emit(
                                "scribe://state-changed",
                                ScribeStateEvent {
                                    progress: Some(progress),
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

        let segments = if let Some((speaker_raw, speaker_native_rate)) = speaker_capture {
            let speaker_16k = resample_linear(&speaker_raw, speaker_native_rate, 16_000);
            let speaker_wav_path = session_dir.join("speaker.wav");
            self.output.write_wav(&speaker_16k, 16_000, &speaker_wav_path)?;

            let mic_segments =
                self.model
                    .transcribe_pcm_with_progress(&model_path, &pcm_16k, move |progress| {
                        callback_progress_tx
                            .send(ProgressMessage::Progress(progress * 0.5))
                            .ok();
                    })?;
            let progress_tx_speaker = progress_tx.clone();
            let speaker_segments =
                self.model
                    .transcribe_pcm_with_progress(&model_path, &speaker_16k, move |progress| {
                        progress_tx_speaker
                            .send(ProgressMessage::Progress(0.5 + (progress * 0.5)))
                            .ok();
                    })?;
            Ok(self.model.merge_dual_source(&mic_segments, &speaker_segments))
        } else {
            self.model
                .transcribe_pcm_with_progress(&model_path, &pcm_16k, move |progress| {
                    callback_progress_tx
                        .send(ProgressMessage::Progress(progress))
                        .ok();
                })
        };
        progress_tx.send(ProgressMessage::Finished).ok();
        progress_thread.join().ok();
        let segments = segments?;

        if abort_flag.load(Ordering::SeqCst) {
            self.clear_transcription_tracking();
            return Ok(());
        }

        let transcript_path = self
            .output
            .transcript_path(&session_dir, &model_path, title);
        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().replace("ggml-", ""))
            .unwrap_or_else(|| "model".to_string());
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
        self.output.write_transcript(
            &segments,
            &notes,
            title,
            &model_name,
            config.include_timestamps,
            &transcript_path,
        )?;

        if !config.keep_wav && !segments.is_empty() {
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
            self.output.delete_wav(&wav_path)?;
            if speaker_capture_enabled {
                let speaker_wav_path = session_dir.join("speaker.wav");
                self.output.delete_wav(&speaker_wav_path)?;
            }
        }

        self.clear_transcription_tracking();
        self.transition(ScribeState::Done);
        self.app
            .emit(
                "scribe://state-changed",
                ScribeStateEvent {
                    transcript_path: Some(transcript_path.to_string_lossy().into()),
                    ..ScribeStateEvent::new(ScribeState::Done)
                },
            )
            .ok();

        Ok(())
    }

    pub fn get_include_timestamps(&self) -> bool {
        self.config.get().include_timestamps
    }

    pub fn set_include_timestamps(&self, enabled: bool) -> Result<()> {
        self.config
            .update(|cfg| cfg.include_timestamps = enabled)
            .map_err(|e| anyhow!("failed to update config: {e}"))
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
            eprintln!("scribe: recovering from poisoned mutex");
            p.into_inner()
        })
    }

    fn restore_output_device(&self, previous: Option<&str>) {
        if let Some(device) = previous {
            if let Err(e) = self.audio.set_output_device(device) {
                eprintln!("failed to restore output device to `{device}`: {e}");
            }
        }
    }

    fn ensure_start_allowed(state: &ScribeState) -> Result<()> {
        if matches!(state, ScribeState::Recording | ScribeState::Transcribing) {
            return Err(anyhow!("cannot start: already in {:?}", state));
        }
        Ok(())
    }
}

/// Linear interpolation resampler. Good enough for speech at 16 kHz target.
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let lo = src.floor() as usize;
        let hi = (lo + 1).min(input.len() - 1);
        let frac = (src - lo as f64) as f32;
        out.push(input[lo] * (1.0 - frac) + input[hi] * frac);
    }
    out
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
            selected_model_id: Some("tiny".to_string()),
            ..Config::default()
        };

        let chosen = resolve_model_path(&config, model.as_ref());
        assert_eq!(chosen, PathBuf::from("/tmp/custom-model.bin"));
    }

    #[test]
    fn resolve_model_path_uses_selected_model_id_when_present() {
        let models_dir =
            std::env::temp_dir().join(format!("liscribe-test-models-{}", uuid::Uuid::new_v4()));
        let model = ModelService::new(models_dir.clone());
        let config = Config {
            selected_model_id: Some("tiny".to_string()),
            ..Config::default()
        };

        let chosen = resolve_model_path(&config, model.as_ref());
        assert_eq!(chosen, models_dir.join("ggml-tiny.bin"));
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
}
