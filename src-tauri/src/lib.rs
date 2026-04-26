mod commands;
mod controllers;
mod platform;
mod services;
mod types;

use std::sync::Arc;
use tauri::Manager;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scribe::scribe_start,
            commands::scribe::scribe_stop_and_save,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
