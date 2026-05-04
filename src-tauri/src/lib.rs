mod commands;
mod controllers;
mod platform;
mod services;
mod types;

use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

pub(crate) const SCRIBE_WINDOW_LABEL: &str = "scribe";
const TRANSCRIBE_WINDOW_LABEL: &str = "transcribe";
const SETTINGS_WINDOW_LABEL: &str = "settings";
pub(crate) const DICTATE_WINDOW_LABEL: &str = "dictate";
const OPEN_SCRIBE_MENU_ID: &str = "open_scribe";
const OPEN_TRANSCRIBE_MENU_ID: &str = "open_transcribe";
const OPEN_SETTINGS_MENU_ID: &str = "open_settings";
const QUIT_MENU_ID: &str = "quit";

const SCRIBE_WINDOW_W: f64 = 800.0;
const SCRIBE_WINDOW_H: f64 = 600.0;
const TRANSCRIBE_WINDOW_W: f64 = 800.0;
const TRANSCRIBE_WINDOW_H: f64 = 600.0;
const SETTINGS_WINDOW_W: f64 = 960.0;
const SETTINGS_WINDOW_H: f64 = 680.0;
const DICTATE_WINDOW_W: f64 = 240.0;
const DICTATE_WINDOW_H: f64 = 48.0;
/// Margin from the right and top edge of the primary monitor.
const DICTATE_MARGIN_RIGHT: f64 = 16.0;
const DICTATE_MARGIN_TOP: f64 = 28.0;

fn resolve_icon_path(app: &tauri::AppHandle, file_name: &str) -> Option<std::path::PathBuf> {
    let resource_icon = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("icons").join(file_name));
    let current_dir = std::env::current_dir().ok();
    let cwd_src_icon = current_dir
        .as_ref()
        .map(|dir| dir.join("src-tauri/icons").join(file_name));
    let cwd_icon = current_dir
        .as_ref()
        .map(|dir| dir.join("icons").join(file_name));

    [resource_icon, cwd_src_icon, cwd_icon]
        .into_iter()
        .flatten()
        .find(|path| path.exists())
}

fn load_icon(app: &tauri::AppHandle, file_name: &str) -> Option<Image<'static>> {
    let path = resolve_icon_path(app, file_name)?;
    Image::from_path(path).ok()
}

fn create_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let open_scribe =
        MenuItem::with_id(app, OPEN_SCRIBE_MENU_ID, "Scribe", true, None::<&str>)?;
    let open_transcribe = MenuItem::with_id(
        app,
        OPEN_TRANSCRIBE_MENU_ID,
        "Transcribe",
        true,
        None::<&str>,
    )?;
    let open_settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_MENU_ID,
        "Settings",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open_scribe, &open_transcribe, &open_settings, &quit],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_SCRIBE_MENU_ID => {
                if let Err(err) = open_scribe_window(app) {
                    eprintln!("failed to open scribe window: {err}");
                }
            }
            OPEN_TRANSCRIBE_MENU_ID => {
                if let Err(err) = open_transcribe_window(app) {
                    eprintln!("failed to open transcribe window: {err}");
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

    #[cfg(not(target_os = "macos"))]
    {
        let preferred_tray_icon = "sf_Transparent_tray_32x32.png";
        if let Some(icon) =
            load_icon(app.handle(), preferred_tray_icon).or_else(|| app.default_window_icon().cloned())
        {
            tray = tray.icon(icon.clone());
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Prefer the generated app icon so tray updates immediately after icon regeneration.
        if let Some(icon) =
            load_icon(app.handle(), "icon.png").or_else(|| app.default_window_icon().cloned())
        {
            tray = tray.icon(icon);
        }
        // Keep original colors; template mode can mask icon changes.
        tray = tray.icon_as_template(false);
    }

    tray.build(app)?;
    Ok(())
}

fn prewarm_scribe_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let url = WebviewUrl::App("index.html".into());
        let mut builder = WebviewWindowBuilder::new(app, SCRIBE_WINDOW_LABEL, url)
            .title("Scribe")
            .inner_size(SCRIBE_WINDOW_W, SCRIBE_WINDOW_H)
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

fn prewarm_transcribe_window(app: &AppHandle) {
    let result: tauri::Result<()> = (|| {
        let url = WebviewUrl::App("?view=transcribe".into());
        let mut builder = WebviewWindowBuilder::new(app, TRANSCRIBE_WINDOW_LABEL, url)
            .title("Transcribe")
            .inner_size(TRANSCRIBE_WINDOW_W, TRANSCRIBE_WINDOW_H)
            .visible(false);
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }
        builder.build()?;
        Ok(())
    })();
    if let Err(err) = result {
        eprintln!("failed to prewarm transcribe window: {err}");
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
        .inner_size(DICTATE_WINDOW_W, DICTATE_WINDOW_H)
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
        SCRIBE_WINDOW_W,
        SCRIBE_WINDOW_H,
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
        SETTINGS_WINDOW_W,
        SETTINGS_WINDOW_H,
    )
}

pub(crate) fn open_transcribe_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    open_or_focus_window(
        app,
        TRANSCRIBE_WINDOW_LABEL,
        "Transcribe",
        WebviewUrl::App("?view=transcribe".into()),
        TRANSCRIBE_WINDOW_W,
        TRANSCRIBE_WINDOW_H,
    )
}

pub(crate) fn open_dictate_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(DICTATE_WINDOW_LABEL) {
        let (x, y) = primary_monitor_dictate_position(app);
        let _ = window.set_position(tauri::LogicalPosition::new(x, y));
        window.show()?;
        return Ok(window);
    }

    let (x, y) = primary_monitor_dictate_position(app);
    let window = WebviewWindowBuilder::new(
        app,
        DICTATE_WINDOW_LABEL,
        WebviewUrl::App("?view=dictate".into()),
    )
    .inner_size(DICTATE_WINDOW_W, DICTATE_WINDOW_H)
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
    let x = width - DICTATE_WINDOW_W - DICTATE_MARGIN_RIGHT;
    let y = DICTATE_MARGIN_TOP;
    (x, y)
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
    let transcribe_input = services::transcribe_input::TranscribeInputService::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
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
            let transcribe_ctrl = controllers::transcribe::TranscribeController::new(
                Arc::clone(&transcribe_input),
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
            app.manage(Arc::clone(&transcribe_ctrl)); // for transcribe commands

            dictate_ctrl.start_key_listener();

            if is_first_run {
                open_settings_window(app.handle())?;
                app.state::<Arc<controllers::settings::SettingsController>>()
                    .complete_onboarding()
                    .ok();
            }
            prewarm_scribe_window(app.handle());
            prewarm_transcribe_window(app.handle());
            prewarm_dictate_window(app.handle());
            // Tao applies Regular activation at launch; `set_dock_visibility(false)` only runs when we
            // call it. Sync once after prewarm so a tray-only start hides the Dock (plist LSUIElement
            // is not sufficient on its own).
            platform::window_impl::sync_activation_policy(app.handle());
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
                // Dictate is a HUD overlay — its close should never affect Dock visibility.
                if window.label() != DICTATE_WINDOW_LABEL {
                    platform::window_impl::sync_activation_policy(window.app_handle());
                }
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
            commands::dictate::dictate_dismiss,
            commands::dictate::dictate_get_history,
            commands::transcribe::transcribe_inspect_inputs,
            commands::transcribe::transcribe_start,
            commands::transcribe::transcribe_open_output,
            commands::transcribe::transcribe_show_window,
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
