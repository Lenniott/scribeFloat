use crate::services::{
    audio::{AudioService, MicSession, WHISPER_SAMPLE_RATE},
    config::ConfigService,
    model::ModelService,
    output::OutputService,
};
use crate::services::audio::resample_linear;
use crate::types::{Config, DictateProcessingStage, DictateState, DictateStateEvent};
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

const DICTATE_AUDIO_LEVEL_EVENT: &str = "dictate://audio-level";
const DICTATE_STATE_EVENT: &str = "dictate://state-changed";

// ── Key tracker constants ────────────────────────────────────────────────────
//
// Two activation modes — mic only opens on the SECOND Left Control press:
//
// • Hold-to-talk: double-tap to open mic; keep second press held ≥
//   HOLD_THRESHOLD_MS then release to stop → transcribe → paste.
//
// • Toggle: double-tap quickly opens mic (both presses < HOLD_THRESHOLD_MS).
//   A third press after TOGGLE_STOP_COOLDOWN_MS stops → transcribe → paste.
//   A third press within the cooldown is ignored (prevents key-repeat noise).
//
// Modifier protection: a first press held ≥ FIRST_PRESS_MAX_MS is treated as
// a modifier combo (Ctrl+C, Ctrl+V, etc.) and resets the state machine to Idle
// without ever opening the mic.

/// Max ms for the first press to count as a tap (not a modifier hold).
const FIRST_PRESS_MAX_MS: u128 = 300;
/// Second press held ≥ this many ms = hold-to-talk mode (release stops).
const HOLD_THRESHOLD_MS: u128 = 500;
/// Max ms from first key-UP to second key-DOWN for double-tap to register.
const DOUBLE_TAP_WINDOW_MS: u128 = 400;
/// Minimum ms after entering toggle-mode before a third tap can stop.
const TOGGLE_STOP_COOLDOWN_MS: u128 = 1000;
/// How long (ms) the Done panel stays visible before auto-dismissing.
const DONE_DISMISS_MS: u64 = 2500;

// ── Key tracker state machine ────────────────────────────────────────────────

#[derive(Debug)]
enum DictateKeyState {
    Idle,
    /// First press is down; waiting to see if it's a tap or a modifier hold.
    FirstPressed { down_at: Instant },
    /// First press was a short tap; waiting for the second press.
    AwaitingSecondTap { up_at: Instant },
    /// Second press is down — mic is now recording.
    SecondHeld { down_at: Instant },
    /// Second press was short; mic is recording in toggle mode.
    ToggleRecording { started_at: Instant },
}

#[derive(Debug, PartialEq)]
pub(crate) enum DictateAction {
    None,
    Start,
    Stop,
}

struct DictateKeyTracker {
    state: DictateKeyState,
}

impl DictateKeyTracker {
    fn new() -> Self {
        Self { state: DictateKeyState::Idle }
    }

    fn on_key_down(&mut self, now: Instant) -> DictateAction {
        match self.state {
            DictateKeyState::Idle => {
                // First press — no action yet; wait to see if it's a tap or modifier hold.
                self.state = DictateKeyState::FirstPressed { down_at: now };
                DictateAction::None
            }
            DictateKeyState::FirstPressed { .. } => {
                // Key-repeat while first press held — ignore.
                DictateAction::None
            }
            DictateKeyState::AwaitingSecondTap { up_at } => {
                if now.duration_since(up_at).as_millis() < DOUBLE_TAP_WINDOW_MS {
                    // Second press within window — open mic.
                    self.state = DictateKeyState::SecondHeld { down_at: now };
                    DictateAction::Start
                } else {
                    // Second press too late — treat as a new first press.
                    self.state = DictateKeyState::FirstPressed { down_at: now };
                    DictateAction::None
                }
            }
            DictateKeyState::SecondHeld { .. } => {
                // Key-repeat while second press held — ignore.
                DictateAction::None
            }
            DictateKeyState::ToggleRecording { started_at } => {
                if now.duration_since(started_at).as_millis() >= TOGGLE_STOP_COOLDOWN_MS {
                    self.state = DictateKeyState::Idle;
                    DictateAction::Stop
                } else {
                    // Within cooldown — ignore to prevent key-repeat false stops.
                    DictateAction::None
                }
            }
        }
    }

    fn on_key_up(&mut self, now: Instant) -> DictateAction {
        match self.state {
            DictateKeyState::FirstPressed { down_at } => {
                let held_ms = now.duration_since(down_at).as_millis();
                if held_ms < FIRST_PRESS_MAX_MS {
                    // Short first press = tap; wait for second.
                    self.state = DictateKeyState::AwaitingSecondTap { up_at: now };
                } else {
                    // Held too long = modifier combo (Ctrl+C, etc.) — reset.
                    self.state = DictateKeyState::Idle;
                }
                DictateAction::None
            }
            DictateKeyState::SecondHeld { down_at } => {
                let held_ms = now.duration_since(down_at).as_millis();
                if held_ms >= HOLD_THRESHOLD_MS {
                    // Long second press = hold-to-talk; release stops.
                    self.state = DictateKeyState::Idle;
                    DictateAction::Stop
                } else {
                    // Short second press = toggle mode; mic keeps recording.
                    self.state = DictateKeyState::ToggleRecording { started_at: now };
                    DictateAction::None
                }
            }
            DictateKeyState::ToggleRecording { .. } => DictateAction::None,
            _ => DictateAction::None,
        }
    }

    /// Called every 50 ms to expire timed states.
    fn check_timeout(&mut self, now: Instant) {
        match self.state {
            DictateKeyState::FirstPressed { down_at } => {
                // Key held longer than FIRST_PRESS_MAX_MS without a key_up yet
                // (e.g. OS held it while composing a modifier chord) — reset.
                if now.duration_since(down_at).as_millis() >= FIRST_PRESS_MAX_MS {
                    self.state = DictateKeyState::Idle;
                }
            }
            DictateKeyState::AwaitingSecondTap { up_at } => {
                if now.duration_since(up_at).as_millis() >= DOUBLE_TAP_WINDOW_MS {
                    // Double-tap window expired — reset silently (no mic was open).
                    self.state = DictateKeyState::Idle;
                }
            }
            _ => {}
        }
    }
}

// ── Controller ───────────────────────────────────────────────────────────────

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

    /// Spawn the global key listener on a background thread.
    /// Must be called once after the controller is created.
    pub fn start_key_listener(self: Arc<Self>) {
        std::thread::spawn(move || {
            let tracker = Arc::new(Mutex::new(DictateKeyTracker::new()));

            // Timeout thread: advances timed states (FirstPressed and AwaitingSecondTap)
            // so the state machine resets to Idle when windows expire.
            {
                let tracker_clone = Arc::clone(&tracker);
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    tracker_clone
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .check_timeout(Instant::now());
                });
            }

            let tracker_main = Arc::clone(&tracker);
            let ctrl_main = Arc::clone(&self);

            if let Err(e) = rdev::listen(move |event| {
                if !crate::platform::dictate_key_matches(&event) {
                    return;
                }
                let action = {
                    let mut t =
                        tracker_main.lock().unwrap_or_else(|p| p.into_inner());
                    match event.event_type {
                        rdev::EventType::KeyPress(_) => t.on_key_down(Instant::now()),
                        rdev::EventType::KeyRelease(_) => t.on_key_up(Instant::now()),
                        _ => DictateAction::None,
                    }
                };
                Self::dispatch_action(Arc::clone(&ctrl_main), action);
            }) {
                eprintln!("dictate: rdev listener stopped: {e:?}");
            }
        });
    }

    fn dispatch_action(this: Arc<Self>, action: DictateAction) {
        match action {
            DictateAction::Start => {
                if this.current_state() != DictateState::Idle {
                    return;
                }
                if let Err(e) = crate::open_dictate_window(&this.app) {
                    eprintln!("dictate: failed to open window: {e}");
                    return;
                }
                if let Err(e) = this.start() {
                    eprintln!("dictate: failed to start: {e}");
                }
            }
            DictateAction::Stop => {
                if let Err(e) = Self::stop_and_transcribe(Arc::clone(&this)) {
                    eprintln!("dictate: failed to stop: {e}");
                }
            }
            DictateAction::None => {}
        }
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

    /// Cancel from Recording or Done state → Idle. Discards audio. Hides window.
    pub fn cancel(&self) -> Result<()> {
        let session = {
            let mut inner = self.lock();
            match inner.state {
                DictateState::Idle => return Ok(()),
                DictateState::Recording | DictateState::Done => {}
                other => {
                    return Err(anyhow!("cannot cancel dictate: state is {:?}", other));
                }
            }
            inner.state = DictateState::Idle;
            let s = inner.session.take();
            self.emit_state_event(&inner);
            s
        };
        drop(session);
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        self.hide_window();
        Ok(())
    }

    /// Dismiss the Done panel → Idle. No-op from any other state.
    pub fn dismiss(&self) {
        let should_hide = {
            let mut inner = self.lock();
            if inner.state != DictateState::Done {
                return;
            }
            inner.state = DictateState::Idle;
            self.emit_state_event(&inner);
            true
        };
        if should_hide {
            self.hide_window();
        }
    }

    /// Transition Recording → Transcribing → Done (or Error/Idle for edge cases).
    /// Returns immediately; all heavy work runs in spawn_blocking.
    pub fn stop_and_transcribe(this: Arc<Self>) -> Result<()> {
        let abort_flag = Arc::new(AtomicBool::new(false));
        let session = {
            let mut inner = this.lock();
            if inner.state != DictateState::Recording {
                return Err(anyhow!("cannot stop dictate: not recording"));
            }
            inner.state = DictateState::Transcribing;
            inner.transcription_abort = Some(Arc::clone(&abort_flag));
            this.app
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        progress: Some(0.0),
                        processing_stage: Some(DictateProcessingStage::LoadingModel),
                        ..DictateStateEvent::new(DictateState::Transcribing)
                    },
                )
                .ok();
            inner.session.take().expect("session exists when Recording")
        };

        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result =
                tokio::task::spawn_blocking(move || ctrl.do_transcription(session, abort_flag))
                    .await;

            match result {
                Ok(Ok(true)) => {
                    // Done: panel visible. Auto-dismiss after DONE_DISMISS_MS.
                    tokio::time::sleep(std::time::Duration::from_millis(DONE_DISMISS_MS)).await;
                    this.auto_dismiss();
                }
                Ok(Ok(false)) => {
                    // Error (too short / no model) or silent abort.
                    // Error state stays visible briefly; Idle state already hid the window.
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    this.auto_dismiss();
                }
                Ok(Err(e)) => {
                    eprintln!("dictate transcription error: {e}");
                    this.set_error_state(e.to_string());
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    this.auto_dismiss();
                }
                Err(e) => {
                    eprintln!("dictate transcription panicked: {e}");
                    this.set_error_state("Transcription crashed unexpectedly.".to_string());
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    this.auto_dismiss();
                }
            }
        });

        Ok(())
    }

    /// Blocking transcription pipeline — runs inside spawn_blocking.
    /// Returns Ok(true) when text was pasted (Done state), Ok(false) when silently idle.
    fn do_transcription(
        &self,
        session: DictateMicSession,
        abort_flag: Arc<AtomicBool>,
    ) -> Result<bool> {
        let config = self.config.get();

        let (raw_pcm, native_rate) = session.mic.stop_and_take();
        let pcm_16k = resample_linear(&raw_pcm, native_rate, WHISPER_SAMPLE_RATE);

        // Whisper rejects very short / silent captures — treat as user spoke too briefly.
        const MIN_PCM_SAMPLES_16K: usize = WHISPER_SAMPLE_RATE as usize / 10; // 100 ms
        if pcm_16k.len() < MIN_PCM_SAMPLES_16K {
            self.set_error_state("Recording too short — try again.".to_string());
            return Ok(false);
        }

        let model_path = resolve_dictate_model_path(&config, &self.model);

        if !self.model.model_available(&model_path) {
            self.set_error_state(
                "No Whisper model available. Download one in Settings → Models.".to_string(),
            );
            return Ok(false);
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
                            progress: Some(p),
                            processing_stage: Some(DictateProcessingStage::TranscribingAudio),
                            ..DictateStateEvent::new(DictateState::Transcribing)
                        },
                    )
                    .ok();
            },
        )?;

        if abort_flag.load(Ordering::SeqCst) {
            self.transition_to_idle();
            return Ok(false);
        }

        if segments.is_empty() {
            self.transition_to_idle();
            return Ok(false);
        }

        let text: String = segments
            .iter()
            .map(|s| s.text.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        // Brief Pasting state so the UI can show the transition.
        {
            let mut inner = self.lock();
            inner.state = DictateState::Pasting;
            inner.transcription_abort = None;
            self.app
                .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(DictateState::Pasting))
                .ok();
        }

        // Write to history log.
        if let Err(e) = self.output.write_dictate_history_entry(&config.save_folder, &text) {
            eprintln!("dictate: failed to write history: {e}");
        }

        // Copy to clipboard first; paste simulation reads from there.
        if let Err(e) = self.app.clipboard().write_text(text.clone()) {
            eprintln!("dictate: failed to write clipboard: {e}");
        }

        if config.dictate_auto_paste {
            if let Err(e) = self.output.paste_text() {
                eprintln!("dictate: paste simulation failed: {e}");
            } else if config.dictate_auto_enter {
                if let Err(e) = self.output.send_enter() {
                    eprintln!("dictate: enter simulation failed: {e}");
                }
            }
        }

        // Transition to Done — window stays visible; auto-dismiss handled by caller.
        {
            let mut inner = self.lock();
            inner.state = DictateState::Done;
            inner.transcription_abort = None;
            self.app
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        text: Some(text),
                        ..DictateStateEvent::new(DictateState::Done)
                    },
                )
                .ok();
        }
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);

        Ok(true)
    }

    pub fn get_history(&self) -> Result<Vec<crate::types::DictateHistoryEntry>, String> {
        let save_folder = self.config.get().save_folder;
        self.output.read_dictate_history(&save_folder).map_err(|e| e.to_string())
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn transition_to_idle(&self) {
        let mut inner = self.lock();
        inner.state = DictateState::Idle;
        inner.transcription_abort = None;
        self.emit_state_event(&inner);
        drop(inner);
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        self.hide_window();
    }

    fn set_error_state(&self, msg: String) {
        let mut inner = self.lock();
        inner.state = DictateState::Error;
        inner.transcription_abort = None;
        self.app
            .emit(
                DICTATE_STATE_EVENT,
                DictateStateEvent {
                    error: Some(msg),
                    ..DictateStateEvent::new(DictateState::Error)
                },
            )
            .ok();
        drop(inner);
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        // Window stays visible; caller schedules auto_dismiss after a delay.
    }

    /// Dismiss the panel if it is in Done or Error state. No-op from any other state.
    fn auto_dismiss(&self) {
        let should_hide = {
            let mut inner = self.lock();
            match inner.state {
                DictateState::Done | DictateState::Error => {
                    inner.state = DictateState::Idle;
                    self.emit_state_event(&inner);
                    true
                }
                _ => false,
            }
        };
        if should_hide {
            self.hide_window();
        }
    }

    fn hide_window(&self) {
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

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| {
            eprintln!("dictate: recovering from poisoned mutex");
            p.into_inner()
        })
    }
}

fn resolve_dictate_model_path(config: &Config, model: &ModelService) -> PathBuf {
    if let Some(id) = &config.dictate_model_id {
        model.model_path_for_id(id).unwrap_or_else(|| model.default_model_path())
    } else if let Some(id) = &config.selected_model_id {
        model.model_path_for_id(id).unwrap_or_else(|| model.default_model_path())
    } else {
        model.default_model_path()
    }
}
