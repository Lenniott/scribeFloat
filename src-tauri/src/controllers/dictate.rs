use crate::services::{
    audio::{AudioService, MicSession, WHISPER_SAMPLE_RATE},
    config::ConfigService,
    model::{model_id_preload_eligible, ModelService},
    output::OutputService,
};
use crate::services::audio::read_wav_mono_f32;
use crate::types::{Config, DictateProcessingStage, DictateState, DictateStateEvent};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
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
    /// Second press is down — wait for HOLD_THRESHOLD_MS before Start (mic not open yet).
    SecondHeldArming { down_at: Instant },
    /// Hold threshold crossed; Start(Hold) already dispatched — release Ctrl → Stop.
    HoldRecordingAwaitingRelease,
    /// Second released early; toggle mode — mic opens on Start(Toggle).
    ToggleRecording { started_at: Instant },
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
    /// Set when cancelling a deferred hold Start before `Recording`; async mic spawn observes this.
    hold_start_cancel: Arc<AtomicBool>,
    /// True from `Dispatch Start(HoldWhileHeld)` until recording starts or abort/finish without recording.
    hold_start_in_flight: Arc<AtomicBool>,
    /// macOS: PID of frontmost app before HUD `show()`, restored before Cmd+V paste (see dictate_focus).
    restore_paste_target_pid: Arc<Mutex<Option<i32>>>,
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
            hold_start_cancel: Arc::new(AtomicBool::new(false)),
            hold_start_in_flight: Arc::new(AtomicBool::new(false)),
            restore_paste_target_pid: Arc::new(Mutex::new(None)),
        })
    }

    fn clear_restore_paste_target_pid(&self) {
        let _ = self.restore_paste_target_pid.lock().map(|mut g| *g = None);
    }

    /// Single main-thread hop: snapshot frontmost app (for paste routing) then `show()` the HUD.
    fn capture_paste_target_then_open_overlay(this: Arc<Self>) -> Arc<Mutex<Result<(), String>>> {
        let open_result = Arc::new(Mutex::new(Ok(())));
        let open_clone = Arc::clone(&open_result);
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
            *open_clone.lock().unwrap() = crate::open_dictate_window(&app_open)
                .map(|_| ())
                .map_err(|e| e.to_string());
        });
        open_result
    }

    /// Spawn the global key listener on a background thread.
    /// Must be called once after the controller is created.
    pub fn start_key_listener(self: Arc<Self>) {
        let tracker = Arc::new(Mutex::new(DictateKeyTracker::new()));

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
                if this.current_state() != DictateState::Idle {
                    return;
                }
                match source {
                    DictateStartSource::Toggle => Self::spawn_dictate_window_and_start(this),
                    DictateStartSource::HoldImmediateStop => {
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
                        eprintln!("[dictate] failed to stop: {e}");
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
            if open_result.lock().unwrap().as_ref().is_err() {
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
                    eprintln!("[dictate] failed to start mic: {e}");
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
                    eprintln!("[dictate] failed to stop after hold-blip: {e}");
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
            eprintln!("[dictate] failed to start mic: {e}");
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
        drop(inner);
        self.spawn_record_start_preload();
        Ok(())
    }

    /// Eagerly load the small models into the shared context cache while recording is in
    /// progress. Mirrors `ScribeController::spawn_record_start_preload`.
    fn spawn_record_start_preload(&self) {
        let cfg = self.config.get();
        let path = match preload_path_for_dictate(&cfg, &self.model) {
            Some(p) => p,
            None => return,
        };
        if !self.model.model_available(&path) {
            return;
        }
        let model = Arc::clone(&self.model);
        tauri::async_runtime::spawn(async move {
            let _ = tokio::task::spawn_blocking(move || {
                if let Err(e) = model.get_or_load_context(&path) {
                    eprintln!("[dictate] record-start preload failed: {e}");
                }
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

    /// Cancel from Recording or Done state → Idle. Discards audio. Hides window.
    pub fn cancel(&self) -> Result<()> {
        self.clear_restore_paste_target_pid();
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
        if let Some(session) = session {
            // Finalize the WAV writer so the file's RIFF header is well-formed, then
            // delete it — cancel discards audio. Without this the temp file would leak
            // every time the user cancels mid-dictate.
            match session.mic.stop_and_finalize() {
                Ok(path) => {
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => eprintln!("[dictate] cancel finalize: {e}"),
            }
        }
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
            self.clear_restore_paste_target_pid();
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
                    this.set_error_state(e.to_string(), None);
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    this.auto_dismiss();
                }
                Err(e) => {
                    eprintln!("[dictate] transcription panicked: {e}");
                    this.set_error_state("Transcription crashed unexpectedly.".to_string(), None);
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

        let wav_path = session.mic.wav_path().to_path_buf();
        session.mic.stop_and_finalize()?;
        let pcm_16k = match read_wav_mono_f32(&wav_path) {
            Ok(pcm) => pcm,
            Err(e) => {
                let salvaged = self.salvage_dictate_wav(&wav_path);
                self.set_error_state(
                    format!("Could not read recording — {e}"),
                    salvaged,
                );
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

        let vad_path = self.model.vad_model_path();
        let vad = self.model.model_available(&vad_path).then_some(vad_path.as_path());
        let app_clone = self.app.clone();
        let segments = match self.model.transcribe_pcm_with_progress(
            &model_path,
            &pcm_16k,
            vad,
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
        ) {
            Ok(segments) => segments,
            Err(e) => {
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

        let text = self.output.format_dictate_text(&segments, &config.replacement_rules);

        {
            let mut inner = self.lock();
            inner.state = DictateState::Pasting;
            inner.transcription_abort = None;
            self.app
                .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(DictateState::Pasting))
                .ok();
        }

        let history_write_failed =
            if let Err(e) = self.output.write_dictate_history_entry(&config.save_folder, &text) {
                eprintln!("[dictate] failed to write history: {e}");
                true
            } else {
                false
            };

        if let Err(e) = self.app.clipboard().write_text(text.clone()) {
            eprintln!("[dictate] failed to write clipboard: {e}");
            self.delete_dictate_wav(&wav_path);
            self.set_error_state(
                format!("Could not write to clipboard — {e}. Transcription: {text}"),
                None,
            );
            return Ok(true);
        }

        let mut paste_failed = false;
        if config.dictate_auto_paste {
            match self.paste_on_main_thread(config.dictate_auto_enter, text.clone()) {
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
        } else {
            self.clear_restore_paste_target_pid();
        }

        self.delete_dictate_wav(&wav_path);

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
                        history_write_failed,
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
        self.clear_restore_paste_target_pid();
        let mut inner = self.lock();
        inner.state = DictateState::Idle;
        inner.transcription_abort = None;
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
        self.app
            .emit(
                DICTATE_STATE_EVENT,
                DictateStateEvent {
                    error: Some(msg),
                    salvaged_wav_path: salvaged_wav_path
                        .map(|p| p.to_string_lossy().into_owned()),
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
                eprintln!("[dictate] failed to salvage wav: {e}");
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
        let restore_pid = self
            .restore_paste_target_pid
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        let (tx, rx) =
            std::sync::mpsc::channel::<(Result<(), String>, Result<(), String>)>();
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
                    if let Err(e) =
                        crate::platform::dictate_focus::activate_pid_for_paste(pid)
                    {
                        eprintln!(
                            "[dictate] could not re-activate target app before paste: {e}"
                        );
                    }
                }
                // Give the OS a moment to settle focus after hide + activate.
                std::thread::sleep(std::time::Duration::from_millis(150));
                // Guard against clipboard hijacking: verify our text is still there
                // before firing the keypress.
                let clipboard_ok = app.clipboard().read_text()
                    .map(|current| current == expected_text)
                    .unwrap_or(false);
                let paste_res = if clipboard_ok {
                    output.paste_text()
                } else {
                    eprintln!("[dictate] clipboard was modified before paste — aborting");
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
            .emit(DICTATE_STATE_EVENT, DictateStateEvent::new(inner.state.clone()))
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
    if let Some(id) = &config.dictate_model_id {
        model.model_path_for_id(id).unwrap_or_else(|| model.default_model_path())
    } else if let Some(id) = &config.selected_model_id {
        model.model_path_for_id(id).unwrap_or_else(|| model.default_model_path())
    } else {
        model.default_model_path()
    }
}

/// Returns the on-disk path for the configured Dictate model **only when it is in the
/// preload allowlist**. Dictate has its own `dictate_model_id` that overrides the global
/// `selected_model_id`; both are checked.
fn preload_path_for_dictate(config: &Config, model: &ModelService) -> Option<PathBuf> {
    let model_id = config
        .dictate_model_id
        .as_deref()
        .or(config.selected_model_id.as_deref())?;
    if !model_id_preload_eligible(model_id) {
        return None;
    }
    model.model_path_for_id(model_id)
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
        ModelService::new(
            std::env::temp_dir().join(format!("liscribe-dictate-test-{}", uuid::Uuid::new_v4())),
        )
    }

    #[test]
    fn dictate_preload_path_prefers_dictate_model_id_over_global() {
        let model = fake_model_service();
        let config = Config {
            dictate_model_id: Some("tiny-en-q5".to_string()),
            selected_model_id: Some("small-en-q5".to_string()),
            ..Config::default()
        };
        let path = preload_path_for_dictate(&config, model.as_ref()).expect("eligible");
        assert!(path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap()
            .contains("tiny"));
    }

    #[test]
    fn dictate_preload_path_falls_back_to_selected_when_no_dictate_override() {
        let model = fake_model_service();
        let config = Config {
            selected_model_id: Some("base-en-q5".to_string()),
            ..Config::default()
        };
        assert!(preload_path_for_dictate(&config, model.as_ref()).is_some());
    }

    #[test]
    fn dictate_preload_path_returns_none_for_larger_models() {
        let model = fake_model_service();
        for id in ["small-en-q5", "medium-en-q5", "large-v3-turbo-q5"] {
            let config = Config {
                dictate_model_id: Some(id.to_string()),
                ..Config::default()
            };
            assert!(
                preload_path_for_dictate(&config, model.as_ref()).is_none(),
                "{id} should not be eligible"
            );
        }
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
    fn second_press_within_window_arms_hold_state() {
        let mut t = DictateKeyTracker::new();
        let first_up = ms_ago(DOUBLE_TAP_WINDOW_MS as u64 - 50);
        t.state = DictateKeyState::AwaitingSecondTap { up_at: first_up };
        assert_eq!(t.on_key_down(Instant::now()), DictateAction::None);
        assert!(matches!(
            t.state,
            DictateKeyState::SecondHeldArming { .. }
        ));
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
    fn second_held_past_threshold_via_keyup_before_tick_emits_hold_immediate_stop()
    {
        let mut t = DictateKeyTracker::new();
        let second_down = ms_ago(HOLD_THRESHOLD_MS as u64 + 15);
        t.state = DictateKeyState::SecondHeldArming { down_at: second_down };
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
        t.state = DictateKeyState::SecondHeldArming { down_at: second_down };
        assert_eq!(
            t.on_key_up(Instant::now()),
            DictateAction::Start(DictateStartSource::Toggle)
        );
        assert!(matches!(
            t.state,
            DictateKeyState::ToggleRecording { .. }
        ));
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
        assert!(matches!(
            t.state,
            DictateKeyState::ToggleRecording { .. }
        ));
    }

    #[test]
    fn keyup_in_toggle_mode_is_ignored() {
        let mut t = DictateKeyTracker::new();
        t.state =
            DictateKeyState::ToggleRecording { started_at: Instant::now() };
        assert_eq!(t.on_key_up(Instant::now()), DictateAction::None);
        assert!(matches!(
            t.state,
            DictateKeyState::ToggleRecording { .. }
        ));
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
        t.state =
            DictateKeyState::SecondHeldArming { down_at: ms_ago(100) };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(
            t.state,
            DictateKeyState::SecondHeldArming { .. }
        ));

        t.state = DictateKeyState::HoldRecordingAwaitingRelease;
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(
            t.state,
            DictateKeyState::HoldRecordingAwaitingRelease
        ));

        t.state = DictateKeyState::ToggleRecording { started_at: ms_ago(9999) };
        assert_eq!(t.check_timeout(Instant::now()), DictateAction::None);
        assert!(matches!(
            t.state,
            DictateKeyState::ToggleRecording { .. }
        ));
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
        assert_eq!(
            t.on_key_down(second_down),
            DictateAction::None
        );

        let arm_moment =
            second_down + Duration::from_millis(HOLD_THRESHOLD_MS as u64 + 10);
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
        assert!(matches!(
            t.state,
            DictateKeyState::ToggleRecording { .. }
        ));

        // Third press after cooldown → Stop.
        let third =
            second_up + Duration::from_millis(TOGGLE_STOP_COOLDOWN_MS as u64 + 10);
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
