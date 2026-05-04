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
/// Mic callbacks run per CoreAudio buffer (often 100+ Hz). Uncapped `emit` → IPC/WebKit repaint thrash (HUD flicker).
const DICTATE_AUDIO_LEVEL_EMIT_MIN_INTERVAL_MS: u128 = 33;

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
        let tracker = Arc::new(Mutex::new(DictateKeyTracker::new()));

        // Timeout thread: advances timed states so the state machine resets
        // to Idle when tap windows expire without a second keypress.
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

        crate::platform::key_listener::start_modifier_listener(move |event| {
            use crate::platform::key_listener::KeyEventKind;
            let action = {
                let mut t = tracker.lock().unwrap_or_else(|p| p.into_inner());
                match event.kind {
                    KeyEventKind::Down => t.on_key_down(Instant::now()),
                    KeyEventKind::Up => t.on_key_up(Instant::now()),
                }
            };
            Self::dispatch_action(Arc::clone(&self), action);
        });
    }

    fn dispatch_action(this: Arc<Self>, action: DictateAction) {
        match action {
            DictateAction::Start => {
                if this.current_state() != DictateState::Idle {
                    return;
                }
                // Open window on the main thread, then start mic on a tokio thread.
                // The rdev callback thread is not safe for CoreAudio stream creation
                // or AppKit window operations on macOS.
                tauri::async_runtime::spawn(async move {
                    let app = this.app.clone();
                    let open_result = std::sync::Arc::new(std::sync::Mutex::new(Ok(())));
                    let open_result_clone = std::sync::Arc::clone(&open_result);
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        *open_result_clone.lock().unwrap() =
                            crate::open_dictate_window(&app2).map(|_| ());
                    });
                    // Give main thread a tick to process the window open.
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    if open_result.lock().unwrap().is_err() {
                        return;
                    }
                    if let Err(e) = this.start() {
                        eprintln!("[dictate] failed to start mic: {e}");
                    }
                });
            }
            DictateAction::Stop => {
                if let Err(e) = Self::stop_and_transcribe(Arc::clone(&this)) {
                    eprintln!("[dictate] failed to stop: {e}");
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
        let last_level_emit = Arc::new(Mutex::new(None::<Instant>));
        let mic = self.audio.start_mic(
            None,
            true,
            Some(Arc::new(move |level| {
                let mut gate = last_level_emit.lock().unwrap_or_else(|p| p.into_inner());
                let now = Instant::now();
                let emit = match *gate {
                    None => true,
                    Some(t) => now.duration_since(t).as_millis() >= DICTATE_AUDIO_LEVEL_EMIT_MIN_INTERVAL_MS,
                };
                if !emit {
                    return;
                }
                *gate = Some(now);
                drop(gate);
                let _ = app.emit(DICTATE_AUDIO_LEVEL_EVENT, level);
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
            match &inner.state {
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

    /// Dismiss the Done or Error panel → Idle. No-op from any other state.
    pub fn dismiss(&self) {
        let should_hide = {
            let mut inner = self.lock();
            if !matches!(inner.state, DictateState::Done | DictateState::Error) {
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
                    tokio::time::sleep(std::time::Duration::from_millis(DONE_DISMISS_MS)).await;
                    this.auto_dismiss();
                }
                Ok(Ok(false)) => {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    this.auto_dismiss();
                }
                Ok(Err(e)) => {
                    this.set_error_state(e.to_string());
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    this.auto_dismiss();
                }
                Err(e) => {
                    eprintln!("[dictate] transcription panicked: {e}");
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

        {
            let mut inner = self.lock();
            inner.state = DictateState::Pasting;
            inner.transcription_abort = None;
            self.app
                .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(DictateState::Pasting))
                .ok();
        }

        if let Err(e) = self.output.write_dictate_history_entry(&config.save_folder, &text) {
            eprintln!("[dictate] failed to write history: {e}");
        }

        if let Err(e) = self.app.clipboard().write_text(text.clone()) {
            eprintln!("[dictate] failed to write clipboard: {e}");
        }

        let mut paste_failed = false;
        if config.dictate_auto_paste {
            match self.paste_on_main_thread(config.dictate_auto_enter) {
                Ok((paste_res, enter_res)) => {
                    if let Err(e) = paste_res {
                        eprintln!("[dictate] paste simulation failed: {e}");
                        paste_failed = true;
                    }
                    if let Err(e) = enter_res {
                        eprintln!("[dictate] enter simulation failed: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("[dictate] paste dispatch failed: {e}");
                    paste_failed = true;
                }
            }
        }

        {
            let mut inner = self.lock();
            inner.state = DictateState::Done;
            inner.transcription_abort = None;
            self.app
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        text: Some(text),
                        paste_failed,
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
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            if let Some(w) = app.get_webview_window(crate::DICTATE_WINDOW_LABEL) {
                // Do NOT call sync_activation_policy — the dictate HUD is not a
                // user-facing content window and its visibility should never affect
                // the Dock icon or trigger an exit when no other windows are open.
                let _ = w.hide();
            }
        });
    }

    /// Run paste (and optionally Enter) on the main thread.
    /// enigo uses CGEventCreateKeyboardEvent which requires the main dispatch queue on macOS.
    /// Hides the dictate window first so the OS restores focus to the previous app before
    /// Cmd+V fires. Called from spawn_blocking, bridged via a sync channel.
    ///
    /// Returns `(paste_result, enter_result)`; `enter_result` is `Ok(())` when Enter was skipped.
    fn paste_on_main_thread(
        &self,
        auto_enter: bool,
    ) -> Result<(Result<(), String>, Result<(), String>), String> {
        let (tx, rx) =
            std::sync::mpsc::channel::<(Result<(), String>, Result<(), String>)>();
        let output = Arc::clone(&self.output);
        let app = self.app.clone();
        self.app
            .run_on_main_thread(move || {
                // Hide the HUD so the previous app regains focus before we simulate Cmd+V.
                if let Some(w) = app.get_webview_window(crate::DICTATE_WINDOW_LABEL) {
                    let _ = w.hide();
                }
                // Give the OS a moment to restore focus to the previously active app.
                std::thread::sleep(std::time::Duration::from_millis(150));
                let paste_res = output.paste_text();
                let enter_res = if paste_res.is_ok() && auto_enter {
                    output.send_enter()
                } else {
                    Ok(())
                };
                let _ = tx.send((paste_res, enter_res));
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())
    }

    fn emit_state_event(&self, inner: &Inner) {
        self.app
            .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(inner.state.clone()))
            .ok();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Helper: produce an Instant offset from now by a given number of milliseconds.
    // Using addition rather than real sleeps keeps the tests instant and deterministic.
    fn ms_ago(ms: u64) -> Instant {
        Instant::now() - Duration::from_millis(ms)
    }

    // ── First press behaviour ────────────────────────────────────────────────

    #[test]
    fn first_keydown_returns_none_and_enters_first_pressed() {
        let mut t = DictateKeyTracker::new();
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::FirstPressed { .. }));
    }

    #[test]
    fn key_repeat_while_first_pressed_returns_none() {
        let mut t = DictateKeyTracker::new();
        t.on_key_down(Instant::now());
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
    }

    #[test]
    fn short_first_press_keyup_enters_awaiting_second_tap() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(50);
        t.on_key_down(down);
        t.on_key_up(Instant::now());
        assert!(matches!(t.state, DictateKeyState::AwaitingSecondTap { .. }));
    }

    #[test]
    fn long_first_press_keyup_resets_to_idle() {
        // Held longer than FIRST_PRESS_MAX_MS (300 ms) — treated as modifier combo.
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(FIRST_PRESS_MAX_MS as u64 + 10);
        t.on_key_down(down);
        t.on_key_up(Instant::now());
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    // ── Double-tap window ────────────────────────────────────────────────────

    #[test]
    fn second_press_within_window_returns_start() {
        let mut t = DictateKeyTracker::new();
        let first_up = ms_ago(DOUBLE_TAP_WINDOW_MS as u64 - 50);
        t.state = DictateKeyState::AwaitingSecondTap { up_at: first_up };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::Start);
        assert!(matches!(t.state, DictateKeyState::SecondHeld { .. }));
    }

    #[test]
    fn second_press_outside_window_treats_as_new_first_press() {
        let mut t = DictateKeyTracker::new();
        let first_up = ms_ago(DOUBLE_TAP_WINDOW_MS as u64 + 50);
        t.state = DictateKeyState::AwaitingSecondTap { up_at: first_up };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::FirstPressed { .. }));
    }

    // ── Hold-to-talk ─────────────────────────────────────────────────────────

    #[test]
    fn long_second_press_keyup_returns_stop_and_goes_idle() {
        let mut t = DictateKeyTracker::new();
        let second_down = ms_ago(HOLD_THRESHOLD_MS as u64 + 10);
        t.state = DictateKeyState::SecondHeld { down_at: second_down };
        assert_eq!(t.on_key_up(Instant::now()), DictateAction::Stop);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn short_second_press_keyup_enters_toggle_recording() {
        let mut t = DictateKeyTracker::new();
        let second_down = ms_ago(HOLD_THRESHOLD_MS as u64 - 100);
        t.state = DictateKeyState::SecondHeld { down_at: second_down };
        assert_eq!(t.on_key_up(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    // ── Toggle mode ──────────────────────────────────────────────────────────

    #[test]
    fn third_press_after_cooldown_returns_stop_and_goes_idle() {
        let mut t = DictateKeyTracker::new();
        let started = ms_ago(TOGGLE_STOP_COOLDOWN_MS as u64 + 10);
        t.state = DictateKeyState::ToggleRecording { started_at: started };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::Stop);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn third_press_within_cooldown_is_ignored() {
        let mut t = DictateKeyTracker::new();
        let started = ms_ago(TOGGLE_STOP_COOLDOWN_MS as u64 - 200);
        t.state = DictateKeyState::ToggleRecording { started_at: started };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    #[test]
    fn keyup_in_toggle_mode_is_ignored() {
        let mut t = DictateKeyTracker::new();
        t.state = DictateKeyState::ToggleRecording { started_at: Instant::now() };
        assert_eq!(t.on_key_up(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    // ── Timeout / check_timeout ──────────────────────────────────────────────

    #[test]
    fn timeout_expires_first_pressed_to_idle() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(FIRST_PRESS_MAX_MS as u64 + 1);
        t.state = DictateKeyState::FirstPressed { down_at: down };
        t.check_timeout(Instant::now());
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn timeout_does_not_expire_recent_first_pressed() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(FIRST_PRESS_MAX_MS as u64 - 50);
        t.state = DictateKeyState::FirstPressed { down_at: down };
        t.check_timeout(Instant::now());
        assert!(matches!(t.state, DictateKeyState::FirstPressed { .. }));
    }

    #[test]
    fn timeout_expires_awaiting_second_tap_to_idle() {
        let mut t = DictateKeyTracker::new();
        let up = ms_ago(DOUBLE_TAP_WINDOW_MS as u64 + 1);
        t.state = DictateKeyState::AwaitingSecondTap { up_at: up };
        t.check_timeout(Instant::now());
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn timeout_does_not_affect_second_held_or_toggle() {
        let mut t = DictateKeyTracker::new();
        t.state = DictateKeyState::SecondHeld { down_at: ms_ago(9999) };
        t.check_timeout(Instant::now());
        assert!(matches!(t.state, DictateKeyState::SecondHeld { .. }));

        t.state = DictateKeyState::ToggleRecording { started_at: ms_ago(9999) };
        t.check_timeout(Instant::now());
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    // ── Full flows ───────────────────────────────────────────────────────────

    #[test]
    fn full_hold_to_talk_flow() {
        let mut t = DictateKeyTracker::new();
        let t0 = Instant::now();

        // First tap: down then quick up.
        assert_eq!(t.on_key_down(t0), DictateAction::None);
        assert_eq!(t.on_key_up(t0 + Duration::from_millis(80)), DictateAction::None);

        // Second press (within double-tap window): down → Start.
        let second_down = t0 + Duration::from_millis(200);
        assert_eq!(t.on_key_down(second_down), DictateAction::Start);

        // Hold the second press past HOLD_THRESHOLD_MS then release → Stop.
        let second_up = second_down + Duration::from_millis(HOLD_THRESHOLD_MS as u64 + 50);
        assert_eq!(t.on_key_up(second_up), DictateAction::Stop);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn full_toggle_flow() {
        let mut t = DictateKeyTracker::new();
        let t0 = Instant::now();

        // First tap.
        assert_eq!(t.on_key_down(t0), DictateAction::None);
        assert_eq!(t.on_key_up(t0 + Duration::from_millis(80)), DictateAction::None);

        // Second press quickly (< HOLD_THRESHOLD_MS) → Start, then up → ToggleRecording.
        let second_down = t0 + Duration::from_millis(200);
        assert_eq!(t.on_key_down(second_down), DictateAction::Start);
        let second_up = second_down + Duration::from_millis(100);
        assert_eq!(t.on_key_up(second_up), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));

        // Third press after cooldown → Stop.
        let third = second_up + Duration::from_millis(TOGGLE_STOP_COOLDOWN_MS as u64 + 10);
        assert_eq!(t.on_key_down(third), DictateAction::Stop);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn modifier_combo_protection_resets_on_long_first_press() {
        // Simulate Ctrl+C: first press held too long (user presses C while holding Ctrl).
        let mut t = DictateKeyTracker::new();
        let t0 = Instant::now();
        t.on_key_down(t0);
        // check_timeout fires after FIRST_PRESS_MAX_MS — simulates the background timer.
        t.check_timeout(t0 + Duration::from_millis(FIRST_PRESS_MAX_MS as u64 + 1));
        assert!(matches!(t.state, DictateKeyState::Idle));
    }
}
