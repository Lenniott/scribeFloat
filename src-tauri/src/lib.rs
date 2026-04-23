mod commands;
mod controllers;
mod services;
mod types;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio = services::audio::AudioService::new();
    let output = services::output::OutputService::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let data_dir = app.path().app_data_dir()?;
            let config = services::config::ConfigService::load(data_dir.join("config.json"))?;

            let models_dir = data_dir.join("models");
            std::fs::create_dir_all(&models_dir)?;
            let model = services::model::ModelService::new(models_dir);

            let ctrl = controllers::scribe::ScribeController::new(
                Arc::clone(&audio),
                Arc::clone(&model),
                Arc::clone(&output),
                Arc::clone(&config),
                app.handle().clone(),
            );

            app.manage(model); // for model commands
            app.manage(config); // for config-aware model commands
            app.manage(ctrl);  // for scribe commands
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scribe::scribe_start,
            commands::scribe::scribe_stop_and_save,
            commands::scribe::scribe_cancel,
            commands::scribe::scribe_get_state,
            commands::scribe::scribe_add_note,
            commands::scribe::scribe_get_include_timestamps,
            commands::scribe::scribe_set_include_timestamps,
            commands::model::model_status,
            commands::model::model_setup_status,
            commands::model::model_list,
            commands::model::model_download_default,
            commands::model::model_download,
            commands::model::model_select,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
