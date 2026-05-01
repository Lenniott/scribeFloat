mod commands;
mod controllers;
mod platform;
mod services;
mod types;

use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub(crate) const SCRIBE_WINDOW_LABEL: &str = "scribe";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const OPEN_SCRIBE_MENU_ID: &str = "open_scribe";
const OPEN_SETTINGS_MENU_ID: &str = "open_settings";
const CLOSE_WINDOWS_MENU_ID: &str = "close_windows";
const QUIT_MENU_ID: &str = "quit";

fn create_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let open_scribe =
        MenuItem::with_id(app, OPEN_SCRIBE_MENU_ID, "Open Scribe", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_MENU_ID,
        "Open Settings",
        true,
        None::<&str>,
    )?;
    let close_windows = MenuItem::with_id(
        app,
        CLOSE_WINDOWS_MENU_ID,
        "Close Windows",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_scribe, &open_settings, &close_windows, &quit])?;

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
            CLOSE_WINDOWS_MENU_ID => close_all_windows(app),
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
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        window
    } else {
        let mut builder = WebviewWindowBuilder::new(app, label, url)
            .title(title)
            .inner_size(width, height);

        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone())?;
        }

        builder.build()?
    };

    platform::window_impl::set_has_visible_windows(app, true);
    Ok(window)
}

fn close_all_windows(app: &AppHandle) {
    for window in app.webview_windows().values() {
        if let Err(err) = window.hide() {
            eprintln!("failed to hide window {}: {err}", window.label());
        }
    }

    platform::window_impl::set_has_visible_windows(app, false);
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

            app.manage(model); // shared model service
            app.manage(config); // shared config service
            app.manage(model_ctrl); // model command orchestration
            app.manage(settings_ctrl); // settings orchestration
            app.manage(ctrl); // for scribe commands
            prewarm_scribe_window(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
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
            commands::scribe::scribe_read_transcript,
            commands::model::model_setup_status,
            commands::model::model_list,
            commands::model::model_download,
            commands::model::model_select,
            commands::settings::settings_get_output_path,
            commands::settings::settings_set_output_path,
            commands::settings::settings_get_hotkeys,
            commands::settings::settings_set_hotkeys,
            commands::settings::settings_get_input_labels,
            commands::settings::settings_set_input_labels,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
