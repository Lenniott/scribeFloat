use crate::controllers::scribe::resample_linear;
use crate::services::{
    audio::{AudioService, MicSession, WHISPER_SAMPLE_RATE},
    config::ConfigService,
    model::ModelService,
    output::OutputService,
};
use crate::types::{DictateState, DictateStateEvent};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

const DICTATE_AUDIO_LEVEL_EVENT: &str = "dictate://audio-level";
const DICTATE_STATE_EVENT: &str = "dictate://state-changed";

struct DictateMicSession {
    mic: MicSession,
}

struct Inner {
    state: DictateState,
    session: Option<DictateMicSession>,
    transcription_abort: Option<Arc<AtomicBool>>,
}

pub struct DictateController {
    inner: Mutex<Inner>,
    audio: Arc<AudioService>,
    model: Arc<ModelService>,
    output: Arc<OutputService>,
    config: Arc<ConfigService>,
    app: AppHandle,
}

impl DictateController {
    pub fn new(
        audio: Arc<AudioService>,
        model: Arc<ModelService>,
        output: Arc<OutputService>,
        config: Arc<ConfigService>,
        app: AppHandle,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                state: DictateState::Idle,
                session: None,
                transcription_abort: None,
            }),
            audio,
            model,
            output,
            config,
            app,
        })
    }

    pub fn current_state(&self) -> DictateState {
        self.lock().state.clone()
    }

    /// Transition Idle → Recording. Opens mic stream, emits audio level events.
    pub fn start(&self) -> Result<()> {
        {
            let inner = self.lock();
            if inner.state != DictateState::Idle {
                return Err(anyhow!("cannot start dictate: state is {:?}", inner.state));
            }
        }

        let app = self.app.clone();
        let mic = self.audio.start_mic(
            None,
            true,
            Some(Arc::new(move |level| {
                app.emit(DICTATE_AUDIO_LEVEL_EVENT, level).ok();
            })),
        )?;

        let mut inner = self.lock();
        if inner.state != DictateState::Idle {
            return Err(anyhow!("cannot start dictate: state changed during mic setup"));
        }
        inner.state = DictateState::Recording;
        inner.session = Some(DictateMicSession { mic });
        self.emit_state_event(&inner);
        Ok(())
    }

    /// Transition Recording → Idle. Discards the audio buffer; no transcription.
    pub fn cancel(&self) -> Result<()> {
        let session = {
            let mut inner = self.lock();
            if inner.state != DictateState::Recording {
                return Err(anyhow!("cannot cancel dictate: not recording"));
            }
            inner.state = DictateState::Idle;
            let s = inner.session.take();
            self.emit_state_event(&inner);
            s
        };
        // Drop the session (stops the mic stream) outside the lock.
        drop(session);
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        Ok(())
    }

    /// Transition Recording → Transcribing → (Pasting) → Idle.
    /// Returns immediately; all heavy work runs in a background spawn_blocking task.
    pub fn stop_and_transcribe(this: Arc<Self>) -> Result<()> {
        let abort_flag = Arc::new(AtomicBool::new(false));
        let session = {
            let mut inner = this.lock();
            if inner.state != DictateState::Recording {
                return Err(anyhow!("cannot stop dictate: not recording"));
            }
            inner.state = DictateState::Transcribing;
            inner.transcription_abort = Some(Arc::clone(&abort_flag));
            this.emit_state_with_progress(&inner, Some(0.0));
            inner.session.take().expect("session exists when Recording")
        };

        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result = tokio::task::spawn_blocking(move || {
                ctrl.do_transcription(session, abort_flag)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    eprintln!("dictate transcription error: {e}");
                    this.set_error_state(e.to_string());
                }
                Err(e) => {
                    eprintln!("dictate transcription panicked: {e}");
                    this.set_error_state("Transcription crashed unexpectedly.".to_string());
                }
            }
        });

        Ok(())
    }

    /// Blocking transcription pipeline — runs inside spawn_blocking.
    fn do_transcription(&self, session: DictateMicSession, abort_flag: Arc<AtomicBool>) -> Result<()> {
        let config = self.config.get();

        let (raw_pcm, native_rate) = session.mic.stop_and_take();
        let pcm_16k = resample_linear(&raw_pcm, native_rate, WHISPER_SAMPLE_RATE);

        // whisper-rs rejects empty / ~no audio with NoSamples; treat very short captures as silence.
        const MIN_PCM_SAMPLES_16K: usize = WHISPER_SAMPLE_RATE as usize / 10;
        if pcm_16k.len() < MIN_PCM_SAMPLES_16K {
            self.transition_to_idle();
            return Ok(());
        }

        // Resolve model path: prefer dictate_model_id, then selected_model_id, then default.
        let model_path: PathBuf = if let Some(id) = &config.dictate_model_id {
            self.model
                .model_path_for_id(id)
                .unwrap_or_else(|| self.model.default_model_path())
        } else if let Some(id) = &config.selected_model_id {
            self.model
                .model_path_for_id(id)
                .unwrap_or_else(|| self.model.default_model_path())
        } else {
            self.model.default_model_path()
        };

        if !self.model.model_available(&model_path) {
            self.transition_to_idle();
            self.app
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        state: DictateState::Error,
                        progress: None,
                        error: Some("No Whisper model available. Please download one in Settings → Models.".to_string()),
                    },
                )
                .ok();
            return Ok(());
        }

        let app_clone = self.app.clone();
        let segments = self.model.transcribe_pcm_with_progress(
            &model_path,
            &pcm_16k,
            move |p| {
                app_clone
                    .emit(
                        DICTATE_STATE_EVENT,
                        DictateStateEvent {
                            state: DictateState::Transcribing,
                            progress: Some(p),
                            error: None,
                        },
                    )
                    .ok();
            },
        )?;

        if abort_flag.load(Ordering::SeqCst) {
            self.transition_to_idle();
            return Ok(());
        }

        if segments.is_empty() {
            self.transition_to_idle();
            return Ok(());
        }

        // Extract plain text (no timestamps).
        let text: String = segments
            .iter()
            .map(|s| s.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        // Emit Pasting state.
        {
            let mut inner = self.lock();
            inner.state = DictateState::Pasting;
            inner.transcription_abort = None;
            self.app
                .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(DictateState::Pasting))
                .ok();
        }

        // Write to history file.
        if let Err(e) = self.output.write_dictate_history_entry(&config.save_folder, &text) {
            eprintln!("dictate: failed to write history: {e}");
        }

        // Copy to clipboard; optional paste inserts at OS focus (e.g. text field in foreground app).
        if let Err(e) = self.app.clipboard().write_text(text.clone()) {
            eprintln!("dictate: failed to write clipboard: {e}");
        }

        if config.dictate_auto_paste {
            if let Err(e) = crate::platform::paste_impl::paste_text() {
                eprintln!("dictate: paste simulation failed: {e}");
            } else if config.dictate_auto_enter {
                if let Err(e) = crate::platform::paste_impl::send_enter() {
                    eprintln!("dictate: enter simulation failed: {e}");
                }
            }
        }

        self.transition_to_idle();
        Ok(())
    }

    pub fn get_history(&self) -> Result<Vec<crate::types::DictateHistoryEntry>, String> {
        let save_folder = self.config.get().save_folder;
        self.output
            .read_dictate_history(&save_folder)
            .map_err(|e| e.to_string())
    }

    fn transition_to_idle(&self) {
        {
            let mut inner = self.lock();
            inner.state = DictateState::Idle;
            inner.transcription_abort = None;
            self.emit_state_event(&inner);
        }
        self.hide_and_reset_level();
    }

    fn set_error_state(&self, msg: String) {
        {
            let mut inner = self.lock();
            inner.state = DictateState::Error;
            inner.transcription_abort = None;
            self.app
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        state: DictateState::Error,
                        progress: None,
                        error: Some(msg),
                    },
                )
                .ok();
        }
        self.hide_and_reset_level();
    }

    fn hide_and_reset_level(&self) {
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        if let Some(w) = self.app.get_webview_window(crate::DICTATE_WINDOW_LABEL) {
            let _ = w.hide();
            crate::platform::window_impl::sync_activation_policy(&self.app);
        }
    }

    fn emit_state_event(&self, inner: &Inner) {
        self.app
            .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(inner.state.clone()))
            .ok();
    }

    fn emit_state_with_progress(&self, inner: &Inner, progress: Option<f32>) {
        self.app
            .emit(
                DICTATE_STATE_EVENT,
                DictateStateEvent { state: inner.state.clone(), progress, error: None },
            )
            .ok();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| {
            eprintln!("dictate: recovering from poisoned mutex");
            p.into_inner()
        })
    }
}
