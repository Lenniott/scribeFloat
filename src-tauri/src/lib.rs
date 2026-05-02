mod commands;
mod controllers;
mod platform;
mod services;
mod types;

use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

pub(crate) const SCRIBE_WINDOW_LABEL: &str = "scribe";
const SETTINGS_WINDOW_LABEL: &str = "settings";
pub(crate) const DICTATE_WINDOW_LABEL: &str = "dictate";
const OPEN_SCRIBE_MENU_ID: &str = "open_scribe";
const OPEN_SETTINGS_MENU_ID: &str = "open_settings";
const QUIT_MENU_ID: &str = "quit";

fn create_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let open_scribe =
        MenuItem::with_id(app, OPEN_SCRIBE_MENU_ID, "Scribe", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_MENU_ID,
        "Settings",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_scribe, &open_settings, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_SCRIBE_MENU_ID => {
                if let Err(err) = open_scribe_window(app) {
                    eprintln!("failed to open scribe window: {err}");
                }
            }
            OPEN_SETTINGS_MENU_ID => {
                if let Err(err) = open_settings_window(app) {
                    eprintln!("failed to open settings window: {err}");
                }
            }
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

fn prewarm_scribe_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let url = WebviewUrl::App("index.html".into());
        let mut builder = WebviewWindowBuilder::new(app, SCRIBE_WINDOW_LABEL, url)
            .title("Scribe")
            .inner_size(800.0, 600.0)
            .visible(false);
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }
        builder.build()?;
        Ok(())
    })();
    if let Err(err) = result {
        eprintln!("failed to prewarm scribe window: {err}");
    }
}

fn prewarm_dictate_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let (x, y) = primary_monitor_dictate_position(app);
        WebviewWindowBuilder::new(
            app,
            DICTATE_WINDOW_LABEL,
            WebviewUrl::App("?view=dictate".into()),
        )
        .inner_size(240.0, 52.0)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(true)
        .position(x, y)
        .visible(false)
        .build()?;
        Ok(())
    })();
    if let Err(err) = result {
        eprintln!("failed to prewarm dictate window: {err}");
    }
}

pub(crate) fn open_scribe_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window = open_or_focus_window(
        app,
        SCRIBE_WINDOW_LABEL,
        "Scribe",
        WebviewUrl::App("index.html".into()),
        800.0,
        600.0,
    )?;
    let _ = window.emit("scribe://open-requested", serde_json::json!({}));
    Ok(window)
}

pub(crate) fn open_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    open_or_focus_window(
        app,
        SETTINGS_WINDOW_LABEL,
        "Settings",
        WebviewUrl::App("?view=settings".into()),
        960.0,
        680.0,
    )
}

pub(crate) fn open_dictate_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(DICTATE_WINDOW_LABEL) {
        let (x, y) = primary_monitor_dictate_position(app);
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        window.show()?;
        window.set_focus()?;
        return Ok(window);
    }

    let (x, y) = primary_monitor_dictate_position(app);
    let window = WebviewWindowBuilder::new(
        app,
        DICTATE_WINDOW_LABEL,
        WebviewUrl::App("?view=dictate".into()),
    )
    .inner_size(240.0, 52.0)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(true)
    .position(x, y)
    .visible(true)
    .build()?;
    Ok(window)
}

fn primary_monitor_dictate_position(app: &AppHandle) -> (f64, f64) {
    let (width, scale) = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let size = m.size();
            let sf = m.scale_factor();
            (size.width as f64 / sf, sf)
        })
        .unwrap_or((1440.0, 1.0));
    let _ = scale;
    let x = width - 240.0 - 16.0;
    let y = 28.0;
    (x, y)
}

// ── Dictate key tracker (global `rdev` listener, macOS: Left Control only) ──
//
// Two activation modes share one mic session:
//
// • Hold-to-talk: key-down opens the mic immediately; releasing after HOLD_THRESHOLD_MS
//   milliseconds runs stop → transcribe → clipboard (→ paste when `dictate_auto_paste` is on).
//
// • Toggle (double-tap then tap to stop): first key-down starts mic. A *short* release
//   (< HOLD_THRESHOLD_MS) waits for a second tap within DOUBLE_TAP_WINDOW_MS (otherwise cancel).
//   Second key-down confirms hands-free toggle while recording stays on. Third key-down, after
//   TOGGLE_STOP_COOLDOWN_MS, runs stop → transcribe → clipboard (→ paste).
//
// Windows uses Alt instead of Left Control (`is_dictate_key`).

/// Key-up after at least this many ms pressed counts as push-to-talk "release stop" vs a quick
/// tap (await possible second tap for toggle). Kept modest so short bursts still stop on release.
const HOLD_THRESHOLD_MS: u128 = 120;
/// Time (ms) from first key-UP to second key-DOWN for a double-tap to register.
const DOUBLE_TAP_WINDOW_MS: u128 = 400;
/// Minimum time (ms) after entering toggle-mode before a third tap can stop recording.
/// Prevents macOS key-repeat from immediately re-triggering stop.
const TOGGLE_STOP_COOLDOWN_MS: u128 = 1000;

#[derive(Debug)]
enum DictateKeyState {
    /// No activity.
    Idle,
    /// Key is currently held down; we haven't decided hold vs. tap yet.
    Held { down_at: Instant },
    /// First tap released quickly; waiting to see if a second tap arrives.
    /// `up_at` is the moment the key was released — the double-tap window
    /// is measured from here, not from the original keydown.
    AwaitingSecondTap { up_at: Instant },
    /// Double-tap confirmed; recording in toggle mode.
    /// `started_at` guards against key-repeat firing an immediate stop.
    ToggleRecording { started_at: Instant },
}

#[derive(Debug, PartialEq)]
enum DictateAction {
    None,
    Start,
    Stop,
    Cancel,
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
                // Start recording immediately; classify hold vs. tap on key-up.
                self.state = DictateKeyState::Held { down_at: now };
                DictateAction::Start
            }
            DictateKeyState::AwaitingSecondTap { up_at } => {
                if now.duration_since(up_at).as_millis() < DOUBLE_TAP_WINDOW_MS {
                    // Second tap within window — switch to toggle mode.
                    self.state =
                        DictateKeyState::ToggleRecording { started_at: now };
                }
                // If the window expired the timeout thread will handle cleanup.
                DictateAction::None
            }
            DictateKeyState::ToggleRecording { started_at } => {
                // Third tap stops toggle recording — but only after cooldown to
                // ignore macOS key-repeat events.
                if now.duration_since(started_at).as_millis() >= TOGGLE_STOP_COOLDOWN_MS {
                    self.state = DictateKeyState::Idle;
                    DictateAction::Stop
                } else {
                    DictateAction::None
                }
            }
            // Key is already held — ignore OS auto-repeat.
            DictateKeyState::Held { .. } => DictateAction::None,
        }
    }

    fn on_key_up(&mut self, now: Instant) -> DictateAction {
        match self.state {
            DictateKeyState::Held { down_at } => {
                let held_ms = now.duration_since(down_at).as_millis();
                if held_ms >= HOLD_THRESHOLD_MS {
                    // Long hold released — stop and transcribe.
                    self.state = DictateKeyState::Idle;
                    DictateAction::Stop
                } else {
                    // Quick tap — wait for a potential second tap.
                    self.state = DictateKeyState::AwaitingSecondTap { up_at: now };
                    DictateAction::None
                }
            }
            // Key-up during toggle recording is the second-tap key-up; recording
            // continues until a third key-down arrives.
            DictateKeyState::ToggleRecording { .. } => DictateAction::None,
            _ => DictateAction::None,
        }
    }

    /// Called every 50 ms from a background thread to time out the double-tap
    /// window. The window is measured from `up_at` (first key release), so
    /// holding the key never triggers a cancel.
    fn check_timeout(&mut self, now: Instant) -> DictateAction {
        if let DictateKeyState::AwaitingSecondTap { up_at } = self.state {
            if now.duration_since(up_at).as_millis() >= DOUBLE_TAP_WINDOW_MS {
                self.state = DictateKeyState::Idle;
                return DictateAction::Cancel;
            }
        }
        DictateAction::None
    }
}

#[cfg(target_os = "macos")]
fn is_dictate_key(event: &rdev::Event) -> bool {
    matches!(
        event.event_type,
        rdev::EventType::KeyPress(rdev::Key::ControlLeft)
            | rdev::EventType::KeyRelease(rdev::Key::ControlLeft)
    )
}

#[cfg(target_os = "windows")]
fn is_dictate_key(event: &rdev::Event) -> bool {
    matches!(
        event.event_type,
        rdev::EventType::KeyPress(rdev::Key::Alt)
            | rdev::EventType::KeyRelease(rdev::Key::Alt)
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_dictate_key(_event: &rdev::Event) -> bool {
    false
}

fn dispatch_dictate_action(
    action: DictateAction,
    app: &AppHandle,
    dictate_ctrl: &Arc<controllers::dictate::DictateController>,
) {
    match action {
        DictateAction::Start => {
            // Spam guard: ignore if not idle (e.g. key fires while transcribing).
            if dictate_ctrl.current_state() != crate::types::DictateState::Idle {
                return;
            }
            if let Err(e) = open_dictate_window(app) {
                eprintln!("dictate: failed to open window: {e}");
                return;
            }
            if let Err(e) = dictate_ctrl.start() {
                eprintln!("dictate: failed to start: {e}");
            }
        }
        DictateAction::Stop => {
            if let Err(e) =
                controllers::dictate::DictateController::stop_and_transcribe(Arc::clone(dictate_ctrl))
            {
                eprintln!("dictate: failed to stop: {e}");
            }
        }
        DictateAction::Cancel => {
            let _ = dictate_ctrl.cancel();
            if let Some(w) = app.get_webview_window(DICTATE_WINDOW_LABEL) {
                let _ = w.hide();
            }
        }
        DictateAction::None => {}
    }
}

fn setup_dictate_key_listener(
    app: AppHandle,
    dictate_ctrl: Arc<controllers::dictate::DictateController>,
) {
    std::thread::spawn(move || {
        let tracker = Arc::new(Mutex::new(DictateKeyTracker::new()));

        // Background thread: cancel recording if double-tap window expires with no second tap.
        {
            let tracker_clone = Arc::clone(&tracker);
            let app_clone = app.clone();
            let ctrl_clone = Arc::clone(&dictate_ctrl);
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let action = tracker_clone.lock().unwrap().check_timeout(Instant::now());
                if action == DictateAction::Cancel {
                    dispatch_dictate_action(action, &app_clone, &ctrl_clone);
                }
            });
        }

        let tracker_main = Arc::clone(&tracker);
        let app_main = app.clone();
        let ctrl_main = Arc::clone(&dictate_ctrl);

        if let Err(e) = rdev::listen(move |event| {
            if !is_dictate_key(&event) {
                return;
            }
            let action = {
                let mut t = tracker_main.lock().unwrap();
                match event.event_type {
                    rdev::EventType::KeyPress(_) => t.on_key_down(Instant::now()),
                    rdev::EventType::KeyRelease(_) => t.on_key_up(Instant::now()),
                    _ => DictateAction::None,
                }
            };
            dispatch_dictate_action(action, &app_main, &ctrl_main);
        }) {
            eprintln!("dictate: rdev listener stopped: {e:?}");
        }
    });
}

/// Show, restore, and focus. On Windows, `show()` applies visibility asynchronously; a deferred
/// `set_focus` runs after so Tao sees `VISIBLE` and can call `SetForegroundWindow`.
fn raise_webview_window(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;

    #[cfg(target_os = "windows")]
    {
        let label = window.label().to_string();
        let app_handle = app.clone();
        app.run_on_main_thread(move || {
            if let Some(w) = app_handle.get_webview_window(&label) {
                let _ = w.set_focus();
            }
        })?;
    }

    #[cfg(not(target_os = "windows"))]
    let _ = app;

    Ok(())
}

fn open_or_focus_window(
    app: &AppHandle,
    label: &str,
    title: &str,
    url: WebviewUrl,
    width: f64,
    height: f64,
) -> tauri::Result<WebviewWindow> {
    let window = if let Some(window) = app.get_webview_window(label) {
        if let Some(icon) = app.default_window_icon() {
            window.set_icon(icon.clone())?;
        }
        raise_webview_window(app, &window)?;
        window
    } else {
        let mut builder = WebviewWindowBuilder::new(app, label, url)
            .title(title)
            .inner_size(width, height);

        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }

        let window = builder.build()?;
        raise_webview_window(app, &window)?;
        window
    };

    platform::window_impl::set_has_visible_windows(app, true);
    Ok(window)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio = services::audio::AudioService::new();
    let output = services::output::OutputService::new();
    let permissions = services::permissions::PermissionsService::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            platform::window_impl::set_has_visible_windows(app.handle(), false);
            create_tray(app)?;

            let data_dir = app.path().app_data_dir()?;
            let config = services::config::ConfigService::load(data_dir.join("config.json"))?;
            let hotkeys = services::hotkeys::HotkeyService::new(
                services::hotkeys::TauriHotkeyRegistrar::new(app.handle().clone()),
            );

            let is_first_run = !config.get().onboarding_complete;

            let models_dir = data_dir.join("models");
            std::fs::create_dir_all(&models_dir)?;
            let model = services::model::ModelService::new(models_dir);
            let model_ctrl =
                controllers::model::ModelController::new(Arc::clone(&model), Arc::clone(&config));
            let settings_ctrl = controllers::settings::SettingsController::new(
                Arc::clone(&config),
                Arc::clone(&hotkeys),
                Arc::clone(&output),
                Arc::clone(&permissions),
                Arc::clone(&audio),
            );
            if let Err(err) = settings_ctrl.rehydrate_hotkeys() {
                eprintln!("hotkey rehydration skipped: {err}");
            }

            let ctrl = controllers::scribe::ScribeController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&config),
                app.handle().clone(),
            );

            let dictate_ctrl = controllers::dictate::DictateController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&config),
                app.handle().clone(),
            );

            app.manage(model); // shared model service
            app.manage(config); // shared config service
            app.manage(model_ctrl); // model command orchestration
            app.manage(settings_ctrl); // settings orchestration
            app.manage(ctrl); // for scribe commands
            app.manage(Arc::clone(&dictate_ctrl)); // for dictate commands

            setup_dictate_key_listener(app.handle().clone(), dictate_ctrl);

            if is_first_run {
                open_settings_window(app.handle())?;
                app.state::<Arc<controllers::settings::SettingsController>>()
                    .complete_onboarding()
                    .ok();
            }
            prewarm_scribe_window(app.handle());
            prewarm_dictate_window(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == SCRIBE_WINDOW_LABEL && matches!(event, WindowEvent::Destroyed) {
                // Release mic/speaker streams if the webview is torn down before invoke(`scribe_cancel`)
                // completes (crash or exceptional teardown — normal close uses hide, not destroy).
                if let Some(ctrl) = window
                    .app_handle()
                    .try_state::<Arc<controllers::scribe::ScribeController>>()
                {
                    let _ = ctrl.cancel();
                }
                platform::window_impl::sync_activation_policy(window.app_handle());
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == SCRIBE_WINDOW_LABEL {
                    api.prevent_close();
                    let _ = window.emit(
                        "scribe://native-close-requested",
                        serde_json::json!({}),
                    );
                    platform::window_impl::sync_activation_policy(window.app_handle());
                    return;
                }

                api.prevent_close();
                if let Err(err) = window.hide() {
                    eprintln!("failed to hide window {}: {err}", window.label());
                }
                platform::window_impl::sync_activation_policy(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::scribe::scribe_start,
            commands::scribe::scribe_stop_and_save,
            commands::scribe::scribe_save_recording_only,
            commands::scribe::scribe_abort_transcription,
            commands::scribe::scribe_destroy_window,
            commands::scribe::scribe_cancel,
            commands::scribe::scribe_add_note,
            commands::scribe::scribe_get_include_timestamps,
            commands::scribe::scribe_set_include_timestamps,
            commands::scribe::scribe_list_input_devices,
            commands::scribe::scribe_list_output_devices,
            commands::scribe::scribe_read_transcript,
            commands::model::model_setup_status,
            commands::model::model_list,
            commands::model::model_download,
            commands::model::model_select,
            commands::model::model_remove,
            commands::settings::settings_get_output_path,
            commands::settings::settings_set_output_path,
            commands::settings::settings_get_hotkeys,
            commands::settings::settings_set_hotkeys,
            commands::settings::settings_get_input_labels,
            commands::settings::settings_set_input_labels,
            commands::settings::settings_get_preferred_audio_devices,
            commands::settings::settings_set_preferred_audio_devices,
            commands::settings::settings_list_output_devices,
            commands::settings::settings_get_scribe_capture_speaker,
            commands::settings::settings_set_scribe_capture_speaker,
            commands::settings::settings_get_open_with_app_path,
            commands::settings::settings_set_open_with_app_path,
            commands::settings::settings_open_transcript,
            commands::settings::settings_get_theme_mode,
            commands::settings::settings_set_theme_mode,
            commands::settings::settings_permissions_status,
            commands::settings::settings_permissions_open,
            commands::settings::settings_permissions_request,
            commands::settings::settings_onboarding_status,
            commands::settings::settings_complete_onboarding,
            commands::settings::settings_reset_onboarding,
            commands::settings::settings_show_window,
            commands::settings::settings_get_dictate_auto_paste,
            commands::settings::settings_set_dictate_auto_paste,
            commands::settings::settings_get_dictate_auto_enter,
            commands::settings::settings_set_dictate_auto_enter,
            commands::settings::settings_get_dictate_model_id,
            commands::settings::settings_set_dictate_model_id,
            commands::dictate::dictate_cancel,
            commands::dictate::dictate_get_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { code, api, .. } = event {
                // Tray-backed app: default event loop exits when the last window is torn down.
                // Programmatic `app.exit(n)` uses `Some(n)` and must not be prevented.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
