mod commands;
mod controllers;
mod services;
mod types;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio = services::audio::AudioService::new();
    let model = services::model::ModelService::new();
    let output = services::output::OutputService::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let config_path = app.path().app_data_dir()?.join("config.json");
            let config = services::config::ConfigService::load(config_path)?;

            let ctrl = controllers::scribe::ScribeController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                config,
                app.handle().clone(),
            );
            app.manage(ctrl);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scribe::scribe_start,
            commands::scribe::scribe_stop_and_save,
            commands::scribe::scribe_cancel,
            commands::scribe::scribe_get_state,
            commands::scribe::scribe_add_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
