use crate::services::{
    audio::{AudioService, MicSession},
    config::ConfigService,
    model::ModelService,
    output::OutputService,
};
use crate::types::{Note, ScribeState, ScribeStateEvent};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

struct ActiveSession {
    mic: MicSession,
    session_dir: PathBuf,
    started_at: Instant,
}

struct Inner {
    state: ScribeState,
    session: Option<ActiveSession>,
    notes: Vec<Note>,
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
            }),
            audio,
            model,
            output,
            config,
            app,
        })
    }

    /// Transition IDLE → RECORDING. Opens mic and creates session directory.
    pub fn start(&self, preferred_mic: Option<String>) -> Result<()> {
        {
            let inner = self.inner.lock().unwrap();
            Self::ensure_start_allowed(&inner.state)?;
        }

        let cfg = self.config.get();
        let session_dir = self.output.make_session_dir(&cfg.save_folder)?;
        let app = self.app.clone();
        let mic = self.audio.start_mic(
            preferred_mic.as_deref(),
            Some(Arc::new(move |level| {
                app.emit("scribe://audio-level", level).ok();
            })),
        )?;

        let mut inner = self.inner.lock().unwrap();
        Self::ensure_start_allowed(&inner.state)?;
        inner.state = ScribeState::Recording;
        inner.session = Some(ActiveSession {
            mic,
            session_dir,
            started_at: Instant::now(),
        });
        inner.notes.clear();
        self.emit_state(&inner);
        Ok(())
    }

    /// Transition RECORDING → IDLE. Discards the audio buffer.
    pub fn cancel(&self) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.state != ScribeState::Recording {
            return Err(anyhow!("cannot cancel: not recording"));
        }
        inner.session = None;
        inner.state = ScribeState::Idle;
        inner.notes.clear();
        self.emit_state(&inner);
        Ok(())
    }

    /// Transition RECORDING → TRANSCRIBING then → DONE / NO_MODEL.
    /// Returns immediately; heavy work runs in a background spawn_blocking task.
    pub fn stop_and_save(this: Arc<Self>, title: Option<String>) -> Result<()> {
        // Extract session under lock then release immediately.
        let (session, notes) = {
            let mut inner = this.inner.lock().unwrap();
            if inner.state != ScribeState::Recording {
                return Err(anyhow!("cannot stop: not recording"));
            }
            inner.state = ScribeState::Transcribing;
            (
                inner.session.take().expect("session exists when Recording"),
                inner.notes.clone(),
            )
        };

        this.app
            .emit("scribe://state-changed", ScribeStateEvent::new(ScribeState::Transcribing))
            .ok();

        let title =
            title.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());

        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result =
                tokio::task::spawn_blocking(move || ctrl.do_transcription(session, notes, &title))
                    .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    eprintln!("transcription error: {e}");
                    {
                        let mut inner = this.inner.lock().unwrap();
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
                Err(e) => eprintln!("transcription task panicked: {e}"),
            }
        });

        Ok(())
    }

    /// Blocking transcription pipeline. Called inside spawn_blocking.
    fn do_transcription(&self, session: ActiveSession, notes: Vec<Note>, title: &str) -> Result<()> {
        let (raw_pcm, native_rate) = session.mic.stop_and_take();

        let pcm_16k = resample_linear(&raw_pcm, native_rate, 16_000);

        let config = self.config.get();
        let wav_path = session.session_dir.join("mic.wav");
        self.output.write_wav(&pcm_16k, 16_000, &wav_path)?;

        // Use configured path, then selected model id, then built-in default path.
        let model_path = if let Some(p) = &config.scribe_model_path {
            PathBuf::from(p)
        } else if let Some(model_id) = &config.selected_model_id {
            self.model
                .model_path_for_id(model_id)
                .unwrap_or_else(|| self.model.default_model_path())
        } else {
            self.model.default_model_path()
        };

        if !self.model.model_available(&model_path) {
            // Model not downloaded yet — keep the WAV and surface the path.
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

        let segments = self.model.transcribe_pcm(&model_path, &pcm_16k)?;

        let transcript_path = self.output.transcript_path(&session.session_dir, &model_path);
        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().replace("ggml-", ""))
            .unwrap_or_else(|| "model".to_string());
        self.output.write_transcript(
            &segments,
            &notes,
            title,
            &model_name,
            config.include_timestamps,
            &transcript_path,
        )?;

        if !config.keep_wav && !segments.is_empty() {
            self.output.delete_wav(&wav_path)?;
        }

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

    pub fn get_state(&self) -> ScribeStateEvent {
        let inner = self.inner.lock().unwrap();
        ScribeStateEvent::new(inner.state.clone())
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
        let mut inner = self.inner.lock().unwrap();
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
        self.inner.lock().unwrap().state = state;
    }

    fn emit_state(&self, inner: &Inner) {
        self.app
            .emit("scribe://state-changed", ScribeStateEvent::new(inner.state.clone()))
            .ok();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_guard_rejects_recording_and_transcribing_states() {
        assert!(ScribeController::ensure_start_allowed(&ScribeState::Idle).is_ok());
        assert!(ScribeController::ensure_start_allowed(&ScribeState::Done).is_ok());
        assert!(ScribeController::ensure_start_allowed(&ScribeState::NoModel).is_ok());

        assert!(ScribeController::ensure_start_allowed(&ScribeState::Recording).is_err());
        assert!(ScribeController::ensure_start_allowed(&ScribeState::Transcribing).is_err());
    }
}
