use crate::services::audio::read_wav_mono_f32;
use crate::services::transcription::{transcribe_capture, CaptureAudio, TranscribeOptions};
use crate::services::{
    audio::{AudioService, MicSession, WHISPER_SAMPLE_RATE},
    config::ConfigService,
    history::HistoryService,
    model::ModelService,
    output::OutputService,
};
use crate::types::{Config, DictateState, DictateStateEvent, HistoryRecord, ProcessingStage};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
// Two activation modes — after a short first tap + second Ctrl press (within DOUBLE_TAP_WINDOW_MS):
//
// • Hold-to-talk: hold the second Ctrl ≥ HOLD_THRESHOLD_MS before the mic opens
//   (timer thread fires Start); release stops → transcribe → paste.
//
// • Toggle: release the second press before HOLD_THRESHOLD_MS — Start on release;
//   a third Ctrl press after TOGGLE_STOP_COOLDOWN_MS stops → transcribe → paste.
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
/// How long (ms) failure Done/Error panels stay visible before auto-dismissing.
const FAILURE_DISMISS_MS: u64 = 800;

/// How long the completed (100%) progress bar stays visible before the window
/// hides — long enough for ProgressBar's capped catch-up (350ms) to finish.
const DICTATE_COMPLETE_HOLD: std::time::Duration = std::time::Duration::from_millis(450);

// ── Key tracker state machine ────────────────────────────────────────────────

#[derive(Debug)]
enum DictateKeyState {
    Idle,
    /// First press is down; waiting to see if it's a tap or a modifier hold.
    FirstPressed {
        down_at: Instant,
    },
    /// First press was a short tap; waiting for the second press.
    AwaitingSecondTap {
        up_at: Instant,
    },
    /// Second press is down — wait for HOLD_THRESHOLD_MS before Start (mic not open yet).
    SecondHeldArming {
        down_at: Instant,
    },
    /// Hold threshold crossed; Start(Hold) already dispatched — release Ctrl → Stop.
    HoldRecordingAwaitingRelease,
    /// Second released early; toggle mode — mic opens on Start(Toggle).
    ToggleRecording {
        started_at: Instant,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DictateStartSource {
    /// Second Ctrl released before hold threshold completed.
    Toggle,
    /// Second Ctrl held ≥ threshold (tick fired Start); Ctrl still held — Stop on Ctrl up.
    HoldWhileHeld,
    /// Held ≥ threshold by wall clock but Ctrl released before the tick arm — open mic once then transcribe immediately.
    HoldImmediateStop,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DictateAction {
    None,
    Start(DictateStartSource),
    Stop,
}

struct DictateKeyTracker {
    state: DictateKeyState,
}

impl DictateKeyTracker {
    fn new() -> Self {
        Self {
            state: DictateKeyState::Idle,
        }
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
                    // Second press within window — arm for hold-or-toggle (mic not yet open).
                    self.state = DictateKeyState::SecondHeldArming { down_at: now };
                    DictateAction::None
                } else {
                    // Second press too late — treat as a new first press.
                    self.state = DictateKeyState::FirstPressed { down_at: now };
                    DictateAction::None
                }
            }
            DictateKeyState::SecondHeldArming { .. } => {
                // Key-repeat while second press held — ignore.
                DictateAction::None
            }
            DictateKeyState::HoldRecordingAwaitingRelease => {
                // Unexpected down while awaiting release — ignore.
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
            DictateKeyState::SecondHeldArming { down_at } => {
                let held_ms = now.duration_since(down_at).as_millis();
                if held_ms < HOLD_THRESHOLD_MS {
                    // Short second press → toggle mode: start on release.
                    self.state = DictateKeyState::ToggleRecording { started_at: now };
                    DictateAction::Start(DictateStartSource::Toggle)
                } else {
                    // Held ≥ threshold by wall clock before tick dispatched Start —
                    // mic opens once then transcription runs immediately.
                    self.state = DictateKeyState::Idle;
                    DictateAction::Start(DictateStartSource::HoldImmediateStop)
                }
            }
            DictateKeyState::HoldRecordingAwaitingRelease => {
                self.state = DictateKeyState::Idle;
                DictateAction::Stop
            }
            DictateKeyState::ToggleRecording { .. } => DictateAction::None,
            _ => DictateAction::None,
        }
    }

    /// Called every ~50 ms: expires tap windows and arms hold-to-talk Start at threshold.
    fn check_timeout(&mut self, now: Instant) -> DictateAction {
        match self.state {
            DictateKeyState::FirstPressed { down_at } => {
                // Key held longer than FIRST_PRESS_MAX_MS without a key_up yet
                // (e.g. OS held it while composing a modifier chord) — reset.
                if now.duration_since(down_at).as_millis() >= FIRST_PRESS_MAX_MS {
                    self.state = DictateKeyState::Idle;
                }
                DictateAction::None
            }
            DictateKeyState::AwaitingSecondTap { up_at } => {
                if now.duration_since(up_at).as_millis() >= DOUBLE_TAP_WINDOW_MS {
                    // Double-tap window expired — reset silently (no mic was open).
                    self.state = DictateKeyState::Idle;
                }
                DictateAction::None
            }
            DictateKeyState::SecondHeldArming { down_at } => {
                if now.duration_since(down_at).as_millis() >= HOLD_THRESHOLD_MS {
                    self.state = DictateKeyState::HoldRecordingAwaitingRelease;
                    DictateAction::Start(DictateStartSource::HoldWhileHeld)
                } else {
                    DictateAction::None
                }
            }
            _ => DictateAction::None,
        }
    }

    /// UI title-bar start — keyboard third-tap stop must see toggle mode.
    fn arm_ui_toggle(&mut self, now: Instant) {
        self.state = DictateKeyState::ToggleRecording { started_at: now };
    }

    fn reset_to_idle(&mut self) {
        self.state = DictateKeyState::Idle;
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
    /// Temp WAV path while Transcribing/Pasting — used to delete on user abort.
    processing_wav_path: Option<PathBuf>,
}

pub struct DictateController {
    inner: Mutex<Inner>,
    audio: Arc<AudioService>,
    model: Arc<ModelService>,
    output: Arc<OutputService>,
    history: Arc<HistoryService>,
    config: Arc<ConfigService>,
    app: AppHandle,
    /// Set when cancelling a deferred hold Start before `Recording`; async mic spawn observes this.
    hold_start_cancel: Arc<AtomicBool>,
    /// True from `Dispatch Start(HoldWhileHeld)` until recording starts or abort/finish without recording.
    hold_start_in_flight: Arc<AtomicBool>,
    /// macOS: PID of frontmost app before HUD `show()`, restored before Cmd+V paste (see dictate_focus).
    restore_paste_target_pid: Arc<Mutex<Option<i32>>>,
    /// Guards against a second simulated paste in the same transcription (e.g. duplicate Stop events).
    paste_once: AtomicBool,
    /// Bumped on new activity so stale auto-dismiss timers cannot hide a new session HUD.
    dismiss_generation: Arc<AtomicU64>,
    /// Shared with the global key listener — kept in sync when dictate starts/stops from the UI.
    key_tracker: Arc<Mutex<DictateKeyTracker>>,
}

impl DictateController {
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
                state: DictateState::Idle,
                session: None,
                transcription_abort: None,
                processing_wav_path: None,
            }),
            audio,
            model,
            output,
            history,
            config,
            app,
            hold_start_cancel: Arc::new(AtomicBool::new(false)),
            hold_start_in_flight: Arc::new(AtomicBool::new(false)),
            restore_paste_target_pid: Arc::new(Mutex::new(None)),
            paste_once: AtomicBool::new(false),
            dismiss_generation: Arc::new(AtomicU64::new(0)),
            key_tracker: Arc::new(Mutex::new(DictateKeyTracker::new())),
        })
    }

    fn arm_key_tracker_toggle(&self) {
        let mut tracker = self.key_tracker.lock().unwrap_or_else(|p| p.into_inner());
        tracker.arm_ui_toggle(Instant::now());
    }

    fn reset_key_tracker(&self) {
        let mut tracker = self.key_tracker.lock().unwrap_or_else(|p| p.into_inner());
        tracker.reset_to_idle();
    }

    fn bump_dismiss_generation(&self) {
        self.dismiss_generation.fetch_add(1, Ordering::SeqCst);
    }

    fn schedule_failure_dismiss(this: Arc<Self>) {
        let gen = this.dismiss_generation.load(Ordering::SeqCst);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(FAILURE_DISMISS_MS)).await;
            if this.dismiss_generation.load(Ordering::SeqCst) != gen {
                return;
            }
            this.auto_dismiss();
        });
    }

    fn clear_restore_paste_target_pid(&self) {
        let _ = self.restore_paste_target_pid.lock().map(|mut g| *g = None);
    }

    /// Single main-thread hop: snapshot frontmost app (for paste routing) then `show()` the HUD.
    fn capture_paste_target_then_open_overlay(this: Arc<Self>) -> Arc<Mutex<Result<(), String>>> {
        let open_result = Arc::new(Mutex::new(Ok(())));
        let open_clone = Arc::clone(&open_result);
        #[cfg(target_os = "macos")]
        let store = Arc::clone(&this.restore_paste_target_pid);
        let app_open = this.app.clone();
        let app_thread = this.app.clone();
        let _ = app_thread.run_on_main_thread(move || {
            #[cfg(target_os = "macos")]
            if let Some(pid) =
                crate::platform::dictate_focus::capture_frontmost_pid_excluding_self()
            {
                let _ = store.lock().map(|mut g| *g = Some(pid));
            }
            *open_clone.lock().unwrap_or_else(|p| p.into_inner()) =
                crate::open_dictate_window(&app_open)
                    .map(|_| ())
                    .map_err(|e| e.to_string());
        });
        open_result
    }

    /// Spawn the global key listener on a background thread.
    /// Must be called once after the controller is created.
    pub fn start_key_listener(self: Arc<Self>) {
        let tracker = Arc::clone(&self.key_tracker);

        // Timeout thread: advances timed states so the state machine resets
        // to Idle when tap windows expire without a second keypress.
        {
            let ctrl = Arc::clone(&self);
            let tracker_clone = Arc::clone(&tracker);
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let action = {
                    tracker_clone
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .check_timeout(Instant::now())
                };
                Self::dispatch_action(Arc::clone(&ctrl), action);
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
            DictateAction::Start(source) => {
                match this.current_state() {
                    DictateState::Idle => {}
                    DictateState::Done | DictateState::Error => {
                        this.bump_dismiss_generation();
                        this.dismiss();
                    }
                    _ => return,
                }
                match source {
                    DictateStartSource::Toggle => Self::spawn_dictate_window_and_start(this),
                    DictateStartSource::HoldImmediateStop => {
                        // Abort any in-flight hold-to-talk open so we don't also Stop later.
                        this.hold_start_cancel.store(true, Ordering::SeqCst);
                        Self::spawn_dictate_hold_immediate_stop(this);
                    }
                    DictateStartSource::HoldWhileHeld => {
                        this.hold_start_cancel.store(false, Ordering::SeqCst);
                        this.hold_start_in_flight.store(true, Ordering::SeqCst);
                        Self::spawn_dictate_hold_while_held(this);
                    }
                }
            }
            DictateAction::Stop => {
                if this.current_state() == DictateState::Recording {
                    if let Err(e) = Self::stop_and_transcribe(Arc::clone(&this)) {
                        tracing::warn!(error = %e, "dictate failed to stop");
                    }
                    return;
                }
                // Hold deferred: user released Ctrl before mic entered Recording OR abort raced open.
                if this.hold_start_in_flight.load(Ordering::SeqCst) {
                    this.hold_start_cancel.store(true, Ordering::SeqCst);
                }
            }
            DictateAction::None => {}
        }
    }

    fn spawn_dictate_window_and_start(this: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            Self::spawn_dictate_window_and_start_inner_async(&this).await;
        });
    }

    fn spawn_dictate_hold_while_held(this: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            let clear_in_flight = || {
                this.hold_start_in_flight.store(false, Ordering::SeqCst);
            };
            let open_result = Self::capture_paste_target_then_open_overlay(Arc::clone(&this));
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if this.hold_start_cancel.load(Ordering::SeqCst) {
                clear_in_flight();
                this.hide_window();
                return;
            }
            if open_result
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .as_ref()
                .is_err()
            {
                clear_in_flight();
                return;
            }
            if this.hold_start_cancel.load(Ordering::SeqCst) {
                clear_in_flight();
                this.hide_window();
                return;
            }
            match this.start() {
                Err(e) => {
                    tracing::error!(error = %e, "dictate failed to start mic (hold)");
                    clear_in_flight();
                }
                Ok(()) => {
                    clear_in_flight();
                }
            }
        });
    }

    fn spawn_dictate_hold_immediate_stop(this: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            Self::spawn_dictate_window_and_start_inner_async(&this).await;
            if this.current_state() == DictateState::Recording {
                if let Err(e) = Self::stop_and_transcribe(Arc::clone(&this)) {
                    tracing::error!(error = %e, "dictate failed to stop after hold-blip");
                }
            }
        });
    }

    async fn spawn_dictate_window_and_start_inner_async(this: &Arc<Self>) {
        let open_result = Self::capture_paste_target_then_open_overlay(Arc::clone(this));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        if open_result.lock().unwrap().as_ref().is_err() {
            return;
        }
        if let Err(e) = this.start() {
            tracing::error!(error = %e, "dictate failed to start mic");
        }
    }

    pub fn current_state(&self) -> DictateState {
        self.lock().state.clone()
    }

    /// Start or stop dictate from the dashboard title-bar button.
    pub fn trigger_toggle(self: &Arc<Self>) {
        match self.current_state() {
            DictateState::Recording => {
                self.reset_key_tracker();
                if let Err(e) = Self::stop_and_transcribe(Arc::clone(self)) {
                    tracing::warn!(error = %e, "dictate UI stop failed");
                }
            }
            DictateState::Idle => {
                self.arm_key_tracker_toggle();
                Self::dispatch_action(
                    Arc::clone(self),
                    DictateAction::Start(DictateStartSource::Toggle),
                );
            }
            DictateState::Done | DictateState::Error => {
                self.bump_dismiss_generation();
                self.dismiss();
                self.arm_key_tracker_toggle();
                Self::dispatch_action(
                    Arc::clone(self),
                    DictateAction::Start(DictateStartSource::Toggle),
                );
            }
            DictateState::Transcribing | DictateState::Pasting => {}
        }
    }

    /// Transition Idle → Recording. Opens mic stream, emits audio level events.
    pub fn start(&self) -> Result<()> {
        self.bump_dismiss_generation();
        {
            let inner = self.lock();
            if inner.state != DictateState::Idle {
                return Err(anyhow!("cannot start dictate: state is {:?}", inner.state));
            }
        }

        let wav_path = dictate_temp_wav_path(&self.app)?;
        let app = self.app.clone();
        let last_level_emit = Arc::new(Mutex::new(None::<Instant>));
        let mic = self.audio.start_mic(
            None,
            true,
            wav_path,
            Some(Arc::new(move |level| {
                let mut gate = last_level_emit.lock().unwrap_or_else(|p| p.into_inner());
                let now = Instant::now();
                let emit = match *gate {
                    None => true,
                    Some(t) => {
                        now.duration_since(t).as_millis()
                            >= DICTATE_AUDIO_LEVEL_EMIT_MIN_INTERVAL_MS
                    }
                };
                if !emit {
                    return;
                }
                *gate = Some(now);
                drop(gate);
                let _ = app.emit(DICTATE_AUDIO_LEVEL_EVENT, level);
            })),
            None,
            None,
        )?;

        let mut inner = self.lock();
        if inner.state != DictateState::Idle {
            return Err(anyhow!(
                "cannot start dictate: state changed during mic setup"
            ));
        }
        inner.state = DictateState::Recording;
        inner.session = Some(DictateMicSession { mic });
        self.emit_state_event(&inner);
        drop(inner);
        self.spawn_record_start_preload();
        Ok(())
    }

    /// Bring the Dictate model fully to ready while the user is speaking, so
    /// stop-and-transcribe starts as a cache hit. Dictate resolves its own (usually
    /// smaller, faster) model via `preload_path_for_dictate` — never the Record
    /// model — and skips the voiceprint extractor because Dictate does no speaker
    /// analysis. The model service's per-path load lock makes a Stop that lands
    /// mid-preload wait for this load rather than duplicate it.
    fn spawn_record_start_preload(&self) {
        let cfg = self.config.get();
        let path = preload_path_for_dictate(&cfg, &self.model);
        let model = Arc::clone(&self.model);
        tauri::async_runtime::spawn(async move {
            let _ = tokio::task::spawn_blocking(move || {
                model.preload_context(&path);
            })
            .await;
        });
    }

    /// Best-effort finalize of an in-progress capture when the app is quitting.
    /// Leaves a checkpointed temp WAV for startup recovery to salvage.
    pub fn finalize_capture_on_shutdown(&self) {
        let session = {
            let mut inner = self.lock();
            if inner.state != DictateState::Recording {
                return;
            }
            inner.state = DictateState::Idle;
            inner.session.take()
        };
        if let Some(session) = session {
            let _ = session.mic.stop_and_finalize();
        }
        self.clear_restore_paste_target_pid();
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
    }

    /// Cancel from Recording → Idle (discards audio), abort Transcribing/Pasting, or dismiss Done.
    pub fn cancel(&self) -> Result<()> {
        match self.current_state() {
            DictateState::Idle => return Ok(()),
            DictateState::Transcribing | DictateState::Pasting => return self.abort_processing(),
            DictateState::Done => {
                self.bump_dismiss_generation();
                self.dismiss();
                return Ok(());
            }
            DictateState::Recording => {}
            DictateState::Error => {
                self.bump_dismiss_generation();
                self.dismiss();
                return Ok(());
            }
        }

        self.bump_dismiss_generation();
        self.clear_restore_paste_target_pid();
        self.reset_key_tracker();
        let session = {
            let mut inner = self.lock();
            inner.state = DictateState::Idle;
            inner.processing_wav_path = None;
            let s = inner.session.take();
            self.emit_state_event(&inner);
            s
        };
        if let Some(session) = session {
            // Finalize the WAV writer so the file's RIFF header is well-formed, then
            // delete it — cancel discards audio. Without this the temp file would leak
            // every time the user cancels mid-dictate.
            match session.mic.stop_and_finalize() {
                Ok(path) => {
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => tracing::debug!(error = %e, "dictate cancel finalize failed"),
            }
        }
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        self.hide_window();
        Ok(())
    }

    /// Abort an in-flight Transcribing or Pasting pipeline. Returns to Idle immediately.
    /// If paste is already running on the main thread, it may still complete (best-effort).
    pub fn abort_processing(&self) -> Result<()> {
        self.bump_dismiss_generation();
        let wav_path = {
            let mut inner = self.lock();
            match &inner.state {
                DictateState::Transcribing | DictateState::Pasting => {}
                other => {
                    return Err(anyhow!(
                        "cannot abort dictate processing: state is {:?}",
                        other
                    ));
                }
            }
            if let Some(flag) = inner.transcription_abort.as_ref() {
                flag.store(true, Ordering::SeqCst);
            }
            let path = inner.processing_wav_path.take();
            inner.state = DictateState::Idle;
            inner.transcription_abort = None;
            self.emit_state_event(&inner);
            path
        };
        if let Some(path) = wav_path {
            self.delete_dictate_wav(&path);
        }
        self.reset_key_tracker();
        self.clear_restore_paste_target_pid();
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        self.hide_window();
        Ok(())
    }

    /// Dismiss the Done or Error panel → Idle. No-op from any other state.
    pub fn dismiss(&self) {
        self.bump_dismiss_generation();
        let should_hide = {
            let mut inner = self.lock();
            if !matches!(inner.state, DictateState::Done | DictateState::Error) {
                return;
            }
            inner.state = DictateState::Idle;
            inner.processing_wav_path = None;
            self.emit_state_event(&inner);
            true
        };
        if should_hide {
            self.reset_key_tracker();
            self.clear_restore_paste_target_pid();
            self.hide_window();
        }
    }

    /// Transition Recording → Transcribing → Done (or Error/Idle for edge cases).
    /// Returns immediately; all heavy work runs in spawn_blocking.
    pub fn stop_and_transcribe(this: Arc<Self>) -> Result<()> {
        this.bump_dismiss_generation();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let session = {
            let mut inner = this.lock();
            if matches!(
                inner.state,
                DictateState::Transcribing | DictateState::Pasting | DictateState::Done
            ) {
                tracing::debug!("dictate ignoring duplicate stop — pipeline already running");
                return Ok(());
            }
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
                        // The model is preloaded during recording; the wait
                        // here is WAV finalize + read, so label it as such.
                        processing_stage: Some(ProcessingStage::PreparingAudio),
                        ..DictateStateEvent::new(DictateState::Transcribing)
                    },
                )
                .ok();
            let session = inner
                .session
                .take()
                .ok_or_else(|| anyhow!("session missing in Recording state"))?;
            inner.processing_wav_path = Some(session.mic.wav_path().to_path_buf());
            session
        };

        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result =
                tokio::task::spawn_blocking(move || ctrl.do_transcription(session, abort_flag))
                    .await;

            match result {
                Ok(Ok(_)) => {
                    if matches!(
                        this.current_state(),
                        DictateState::Done | DictateState::Error
                    ) {
                        Self::schedule_failure_dismiss(Arc::clone(&this));
                    }
                }
                Ok(Err(e)) => {
                    this.set_error_state(e.to_string(), None);
                    Self::schedule_failure_dismiss(Arc::clone(&this));
                }
                Err(e) => {
                    tracing::error!(error = %e, "dictate transcription panicked");
                    this.set_error_state("Transcription crashed unexpectedly.".to_string(), None);
                    Self::schedule_failure_dismiss(Arc::clone(&this));
                }
            }
        });

        Ok(())
    }

    /// Blocking transcription pipeline — runs inside spawn_blocking.
    /// Returns Ok(true) when a failure Done panel was shown; Ok(false) when silently idle.
    fn do_transcription(
        &self,
        session: DictateMicSession,
        abort_flag: Arc<AtomicBool>,
    ) -> Result<bool> {
        let config = self.config.get();

        let wav_path = session.mic.wav_path().to_path_buf();
        session.mic.stop_and_finalize()?;
        let pcm_16k = match read_wav_mono_f32(&wav_path) {
            Ok(pcm) => pcm,
            Err(e) => {
                let salvaged = self.salvage_dictate_wav(&wav_path);
                self.set_error_state(format!("Could not read recording — {e}"), salvaged);
                return Ok(false);
            }
        };

        const MIN_PCM_SAMPLES_16K: usize = WHISPER_SAMPLE_RATE as usize / 10; // 100 ms
        if pcm_16k.len() < MIN_PCM_SAMPLES_16K {
            self.delete_dictate_wav(&wav_path);
            self.set_error_state("Recording too short — try again.".to_string(), None);
            return Ok(false);
        }

        let model_path = resolve_dictate_model_path(&config, &self.model);

        if !self.model.model_available(&model_path) {
            let salvaged = self.salvage_dictate_wav(&wav_path);
            self.set_error_state(
                "No Whisper model available. Download one in Settings → Models.".to_string(),
                salvaged,
            );
            return Ok(false);
        }

        let app_clone = self.app.clone();
        let progress_reporter = move |p: f32| {
            app_clone
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        progress: Some(p),
                        processing_stage: Some(ProcessingStage::TranscribingAudio),
                        ..DictateStateEvent::new(DictateState::Transcribing)
                    },
                )
                .ok();
        };
        let segments = match transcribe_capture(
            &self.model,
            CaptureAudio {
                mic_pcm_16k: &pcm_16k,
                speaker_pcm_16k: None,
            },
            TranscribeOptions {
                model_path: &model_path,
                source: "dictate",
                abort: None,
                on_model_loaded: None,
            },
            progress_reporter,
        ) {
            Ok(segments) => segments,
            Err(e) => {
                if abort_flag.load(Ordering::SeqCst) {
                    self.delete_dictate_wav(&wav_path);
                    self.transition_to_idle();
                    return Ok(false);
                }
                let salvaged = self.salvage_dictate_wav(&wav_path);
                self.set_error_state(e.to_string(), salvaged);
                return Ok(false);
            }
        };

        if abort_flag.load(Ordering::SeqCst) {
            self.delete_dictate_wav(&wav_path);
            self.transition_to_idle();
            return Ok(false);
        }

        if segments.is_empty() {
            self.delete_dictate_wav(&wav_path);
            self.transition_to_idle();
            return Ok(false);
        }

        let text = self.output.format_dictate_text(&segments);

        if abort_flag.load(Ordering::SeqCst) {
            self.delete_dictate_wav(&wav_path);
            self.transition_to_idle();
            return Ok(false);
        }

        {
            let mut inner = self.lock();
            inner.state = DictateState::Pasting;
            // Terminal progress: the bar must reach 100% before the window
            // hides, even when Whisper's sparse ticks never got there.
            self.app
                .emit(
                    DICTATE_STATE_EVENT,
                    DictateStateEvent {
                        progress: Some(1.0),
                        processing_stage: Some(ProcessingStage::TranscribingAudio),
                        ..DictateStateEvent::new(DictateState::Pasting)
                    },
                )
                .ok();
        }

        if abort_flag.load(Ordering::SeqCst) {
            self.delete_dictate_wav(&wav_path);
            self.transition_to_idle();
            return Ok(false);
        }

        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().replace("ggml-", ""))
            .unwrap_or_else(|| "model".to_string());
        let record = HistoryRecord::from_dictate(&segments, &text, model_name);
        let history_write_failed = if let Err(e) = self.history.append(&config.save_folder, record)
        {
            tracing::warn!(error = %e, "dictate failed to write history");
            true
        } else {
            self.app.emit("note://item-added", ()).ok();
            false
        };

        if let Err(e) = self.app.clipboard().write_text(text.clone()) {
            tracing::error!(error = %e, "dictate failed to write clipboard");
            self.delete_dictate_wav(&wav_path);
            self.set_error_state(
                format!("Could not write to clipboard — {e}. Transcription: {text}"),
                None,
            );
            return Ok(true);
        }

        let mut paste_failed = false;
        if config.dictate_auto_paste {
            if self
                .paste_once
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                tracing::debug!("dictate skipping duplicate paste in same session");
            } else {
                match self.paste_on_main_thread(config.dictate_auto_enter, text.clone()) {
                    Ok((paste_res, enter_res)) => {
                        if let Err(e) = paste_res {
                            tracing::error!(error = %e, "dictate paste simulation failed");
                            paste_failed = true;
                        }
                        if let Err(e) = enter_res {
                            tracing::error!(error = %e, "dictate enter simulation failed");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "dictate paste dispatch failed");
                        paste_failed = true;
                    }
                }
                self.paste_once.store(false, Ordering::SeqCst);
            }
        } else {
            self.clear_restore_paste_target_pid();
        }

        self.delete_dictate_wav(&wav_path);

        if paste_failed {
            {
                let mut inner = self.lock();
                inner.state = DictateState::Done;
                inner.transcription_abort = None;
                inner.processing_wav_path = None;
                self.app
                    .emit(
                        DICTATE_STATE_EVENT,
                        DictateStateEvent {
                            text: Some(text),
                            paste_failed: true,
                            history_write_failed,
                            ..DictateStateEvent::new(DictateState::Done)
                        },
                    )
                    .ok();
            }
            let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
            return Ok(true);
        }

        // Hold the completed bar briefly — hiding the window on the same tick
        // as the terminal progress event means the user never sees 100%.
        std::thread::sleep(DICTATE_COMPLETE_HOLD);
        self.transition_to_idle();
        Ok(false)
    }

    pub fn get_history(&self) -> Result<Vec<crate::types::DictateHistoryEntry>, String> {
        let save_folder = self.config.get().save_folder;
        self.output
            .read_dictate_history(&save_folder)
            .map_err(|e| e.to_string())
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn transition_to_idle(&self) {
        self.clear_restore_paste_target_pid();
        let mut inner = self.lock();
        inner.state = DictateState::Idle;
        inner.transcription_abort = None;
        inner.processing_wav_path = None;
        self.emit_state_event(&inner);
        drop(inner);
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        self.hide_window();
    }

    fn set_error_state(&self, msg: String, salvaged_wav_path: Option<PathBuf>) {
        self.clear_restore_paste_target_pid();
        let mut inner = self.lock();
        inner.state = DictateState::Error;
        inner.transcription_abort = None;
        inner.processing_wav_path = None;
        self.app
            .emit(
                DICTATE_STATE_EVENT,
                DictateStateEvent {
                    error: Some(msg),
                    salvaged_wav_path: salvaged_wav_path.map(|p| p.to_string_lossy().into_owned()),
                    ..DictateStateEvent::new(DictateState::Error)
                },
            )
            .ok();
        drop(inner);
        let _ = self.app.emit(DICTATE_AUDIO_LEVEL_EVENT, 0.0_f32);
        // Window stays visible; caller schedules auto_dismiss after a delay.
    }

    fn delete_dictate_wav(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    fn salvage_dictate_wav(&self, path: &Path) -> Option<PathBuf> {
        match self
            .output
            .salvage_dictate_wav(&self.config.get().save_folder, path)
        {
            Ok(dest) => Some(dest),
            Err(e) => {
                tracing::warn!(error = %e, "dictate failed to salvage wav");
                self.delete_dictate_wav(path);
                None
            }
        }
    }

    /// Dismiss the panel if it is in Done or Error state. No-op from any other state.
    fn auto_dismiss(&self) {
        let should_hide = {
            let mut inner = self.lock();
            match inner.state {
                DictateState::Done | DictateState::Error => {
                    inner.state = DictateState::Idle;
                    inner.processing_wav_path = None;
                    self.emit_state_event(&inner);
                    true
                }
                _ => false,
            }
        };
        if should_hide {
            self.clear_restore_paste_target_pid();
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
    /// Hides the dictate window first, then (macOS) re-activates the app that was frontmost
    /// before the HUD was shown, then simulates Cmd/Ctrl+V. Called from spawn_blocking.
    ///
    /// Returns `(paste_result, enter_result)`; `enter_result` is `Ok(())` when Enter was skipped.
    #[allow(clippy::type_complexity)]
    fn paste_on_main_thread(
        &self,
        auto_enter: bool,
        expected_text: String,
    ) -> Result<(Result<(), String>, Result<(), String>), String> {
        #[cfg(target_os = "macos")]
        let restore_pid = self
            .restore_paste_target_pid
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let (tx, rx) = std::sync::mpsc::channel::<(Result<(), String>, Result<(), String>)>();
        let output = Arc::clone(&self.output);
        let app = self.app.clone();
        self.app
            .run_on_main_thread(move || {
                // Hide the HUD so the OS can return focus elsewhere before we simulate Cmd+V.
                if let Some(w) = app.get_webview_window(crate::DICTATE_WINDOW_LABEL) {
                    let _ = w.hide();
                }
                #[cfg(target_os = "macos")]
                if let Some(pid) = restore_pid {
                    if let Err(e) = crate::platform::dictate_focus::activate_pid_for_paste(pid) {
                        tracing::warn!(error = %e, "could not re-activate target app before paste");
                    }
                }
                // Give the OS a moment to settle focus after hide + activate.
                std::thread::sleep(std::time::Duration::from_millis(150));
                // Guard against clipboard hijacking: verify our text is still there
                // before firing the keypress.
                let clipboard_ok = app
                    .clipboard()
                    .read_text()
                    .map(|current| current == expected_text)
                    .unwrap_or(false);
                let paste_res = if clipboard_ok {
                    output.paste_text()
                } else {
                    tracing::debug!("clipboard was modified before paste — aborting");
                    Err("Clipboard was modified by another process before paste".to_string())
                };
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
            .emit(
                DICTATE_STATE_EVENT,
                DictateStateEvent::new(inner.state.clone()),
            )
            .ok();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// Location of the dictate capture's temp WAV. Lives under the app's local data dir so
/// it's covered by the same sandbox/permissions as `config.json`. A UUID-based name
/// avoids any chance of collision with a stale file from a previous crashed run.
fn dictate_temp_wav_path(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| anyhow!("failed to resolve app local data dir: {e}"))?
        .join("dictate_temp");
    std::fs::create_dir_all(&dir).map_err(|e| anyhow!("create dictate temp dir: {e}"))?;
    Ok(dir.join(format!("{}.wav", uuid::Uuid::new_v4())))
}

fn resolve_dictate_model_path(config: &Config, model: &ModelService) -> PathBuf {
    if let Some(id) = &config.selected_model_id {
        if let Some(path) = model.model_path_for_id(id) {
            if model.model_available(&path) {
                return path;
            }
        }
    }

    if let Some(path) = &config.scribe_model_path {
        let path = PathBuf::from(path);
        if model.model_available(&path) {
            return path;
        }
    }

    model.default_model_path()
}

/// Path of the model that Dictate will use on stop — same resolution as transcription.
fn preload_path_for_dictate(config: &Config, model: &ModelService) -> PathBuf {
    resolve_dictate_model_path(config, model)
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

    // ── Preload eligibility ──────────────────────────────────────────────────

    fn fake_model_service() -> Arc<ModelService> {
        let dir =
            std::env::temp_dir().join(format!("liscribe-dictate-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create model dir");
        ModelService::new(dir)
    }

    fn write_fake_model(model: &ModelService, id: &str) -> PathBuf {
        let path = model.model_path_for_id(id).unwrap();
        std::fs::write(&path, [1, 2, 3]).expect("write model");
        path
    }

    #[test]
    fn dictate_preload_path_uses_selected_model_id() {
        let model = fake_model_service();
        let small_path = write_fake_model(model.as_ref(), "small-en-q5");
        let config = Config {
            selected_model_id: Some("small-en-q5".to_string()),
            ..Config::default()
        };
        let path = preload_path_for_dictate(&config, model.as_ref());
        assert_eq!(path, small_path);
    }

    #[test]
    fn dictate_preload_path_falls_back_to_scribe_model_path() {
        let model = fake_model_service();
        let custom_path = model.default_model_path().with_file_name("custom.bin");
        std::fs::write(&custom_path, [1, 2, 3]).expect("write model");
        let config = Config {
            scribe_model_path: Some(custom_path.to_string_lossy().to_string()),
            ..Config::default()
        };

        let path = preload_path_for_dictate(&config, model.as_ref());

        assert_eq!(path, custom_path);
    }

    #[test]
    fn dictate_preload_path_ignores_selection_pointing_at_removed_catalog_entry() {
        let model = fake_model_service();
        let small_path = write_fake_model(model.as_ref(), "small-en-q5");
        let config = Config {
            selected_model_id: Some("tiny-en-q5".to_string()),
            scribe_model_path: Some(small_path.to_string_lossy().to_string()),
            ..Config::default()
        };

        let path = preload_path_for_dictate(&config, model.as_ref());

        assert_eq!(path, small_path);
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

    /// AC 0048-1: A short first-press tap (key down then up within FIRST_PRESS_MAX_MS)
    /// must return DictateAction::None — no recording starts on a single brief press.
    #[test]
    fn short_tap_returns_no_action() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(FIRST_PRESS_MAX_MS as u64 - 50); // well within limit
        let action_down = t.on_key_down(down);
        let action_up = t.on_key_up(Instant::now());
        assert_eq!(action_down, DictateAction::None);
        assert_eq!(action_up, DictateAction::None);
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
    fn second_press_within_window_arms_hold_state() {
        let mut t = DictateKeyTracker::new();
        let first_up = ms_ago(DOUBLE_TAP_WINDOW_MS as u64 - 50);
        t.state = DictateKeyState::AwaitingSecondTap { up_at: first_up };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::SecondHeldArming { .. }));
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
    fn hold_threshold_tick_emits_start_while_key_still_down() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(HOLD_THRESHOLD_MS as u64 + 1);
        t.state = DictateKeyState::SecondHeldArming { down_at: down };
        assert_eq!(
            t.check_timeout(Instant::now()),
            DictateAction::Start(DictateStartSource::HoldWhileHeld)
        );
        assert!(matches!(
            t.state,
            DictateKeyState::HoldRecordingAwaitingRelease
        ));
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
    }

    #[test]
    fn hold_after_threshold_release_returns_stop_and_goes_idle() {
        let mut t = DictateKeyTracker::new();
        t.state = DictateKeyState::HoldRecordingAwaitingRelease;
        assert_eq!(t.on_key_up(Instant::now()), DictateAction::Stop);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn second_held_past_threshold_via_keyup_before_tick_emits_hold_immediate_stop() {
        let mut t = DictateKeyTracker::new();
        let second_down = ms_ago(HOLD_THRESHOLD_MS as u64 + 15);
        t.state = DictateKeyState::SecondHeldArming {
            down_at: second_down,
        };
        assert_eq!(
            t.on_key_up(Instant::now()),
            DictateAction::Start(DictateStartSource::HoldImmediateStop)
        );
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn short_second_press_keyup_emits_start_toggle() {
        let mut t = DictateKeyTracker::new();
        let second_down = ms_ago(HOLD_THRESHOLD_MS as u64 - 100);
        t.state = DictateKeyState::SecondHeldArming {
            down_at: second_down,
        };
        assert_eq!(
            t.on_key_up(Instant::now()),
            DictateAction::Start(DictateStartSource::Toggle)
        );
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    // ── Toggle mode ──────────────────────────────────────────────────────────

    #[test]
    fn third_press_after_cooldown_returns_stop_and_goes_idle() {
        let mut t = DictateKeyTracker::new();
        let started = ms_ago(TOGGLE_STOP_COOLDOWN_MS as u64 + 10);
        t.state = DictateKeyState::ToggleRecording {
            started_at: started,
        };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::Stop);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn third_press_within_cooldown_is_ignored() {
        let mut t = DictateKeyTracker::new();
        let started = ms_ago(TOGGLE_STOP_COOLDOWN_MS as u64 - 200);
        t.state = DictateKeyState::ToggleRecording {
            started_at: started,
        };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    #[test]
    fn keyup_in_toggle_mode_is_ignored() {
        let mut t = DictateKeyTracker::new();
        t.state = DictateKeyState::ToggleRecording {
            started_at: Instant::now(),
        };
        assert_eq!(t.on_key_up(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    // ── Timeout / check_timeout ──────────────────────────────────────────────

    #[test]
    fn timeout_expires_first_pressed_to_idle() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(FIRST_PRESS_MAX_MS as u64 + 1);
        t.state = DictateKeyState::FirstPressed { down_at: down };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn timeout_does_not_expire_recent_first_pressed() {
        let mut t = DictateKeyTracker::new();
        let down = ms_ago(FIRST_PRESS_MAX_MS as u64 - 50);
        t.state = DictateKeyState::FirstPressed { down_at: down };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::FirstPressed { .. }));
    }

    #[test]
    fn timeout_expires_awaiting_second_tap_to_idle() {
        let mut t = DictateKeyTracker::new();
        let up = ms_ago(DOUBLE_TAP_WINDOW_MS as u64 + 1);
        t.state = DictateKeyState::AwaitingSecondTap { up_at: up };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn timeout_does_not_affect_armed_hold_or_toggle() {
        let mut t = DictateKeyTracker::new();
        t.state = DictateKeyState::SecondHeldArming {
            down_at: ms_ago(100),
        };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::SecondHeldArming { .. }));

        t.state = DictateKeyState::HoldRecordingAwaitingRelease;
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(
            t.state,
            DictateKeyState::HoldRecordingAwaitingRelease
        ));

        t.state = DictateKeyState::ToggleRecording {
            started_at: ms_ago(9999),
        };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(t.state, DictateKeyState::ToggleRecording { .. }));
    }

    // ── Full flows ───────────────────────────────────────────────────────────

    #[test]
    fn full_hold_to_talk_flow() {
        let mut t = DictateKeyTracker::new();
        let t0 = Instant::now();

        // First tap: down then quick up.
        assert_eq!(t.on_key_down(t0), DictateAction::None);
        assert_eq!(
            t.on_key_up(t0 + Duration::from_millis(80)),
            DictateAction::None
        );

        // Second press (within window): mic arms only after hold threshold crossing.
        let second_down = t0 + Duration::from_millis(200);
        assert_eq!(t.on_key_down(second_down), DictateAction::None);

        let arm_moment = second_down + Duration::from_millis(HOLD_THRESHOLD_MS as u64 + 10);
        assert_eq!(
            t.check_timeout(arm_moment),
            DictateAction::Start(DictateStartSource::HoldWhileHeld)
        );

        assert_eq!(
            t.on_key_up(arm_moment + Duration::from_millis(120)),
            DictateAction::Stop
        );
        assert!(matches!(t.state, DictateKeyState::Idle));
    }

    #[test]
    fn full_toggle_flow() {
        let mut t = DictateKeyTracker::new();
        let t0 = Instant::now();

        // First tap.
        assert_eq!(t.on_key_down(t0), DictateAction::None);
        assert_eq!(
            t.on_key_up(t0 + Duration::from_millis(80)),
            DictateAction::None
        );

        // Short second press (< HOLD_THRESHOLD_MS) → Toggle Start on release.
        let second_down = t0 + Duration::from_millis(200);
        assert_eq!(t.on_key_down(second_down), DictateAction::None);
        let second_up = second_down + Duration::from_millis(100);
        assert_eq!(
            t.on_key_up(second_up),
            DictateAction::Start(DictateStartSource::Toggle)
        );
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
        assert_eq!(
            t.check_timeout(t0 + Duration::from_millis(FIRST_PRESS_MAX_MS as u64 + 1)),
            DictateAction::None
        );
        assert!(matches!(t.state, DictateKeyState::Idle));
    }
}
