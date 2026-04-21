use crate::services::model::ModelService;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Whether the default small model is present on disk.
#[tauri::command]
pub fn model_status(model: State<'_, Arc<ModelService>>) -> bool {
    model.default_model_ready()
}

/// Trigger a download of the default small model if not already present.
/// Returns immediately; progress arrives via `model://download-progress` events.
#[tauri::command]
pub fn model_download_default(
    model: State<'_, Arc<ModelService>>,
    app: AppHandle,
) -> Result<(), String> {
    if model.default_model_ready() {
        return Ok(());
    }
    let m = Arc::clone(&model);
    tokio::spawn(async move {
        if let Err(e) = m.download_default(&app).await {
            eprintln!("model download failed: {e}");
            app.emit("model://download-error", e.to_string()).ok();
        }
    });
    Ok(())
}
