use crate::services::config::ConfigService;
use crate::services::model::ModelService;
use crate::types::ModelListItem;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Whether the default small model is present on disk.
#[tauri::command]
pub fn model_status(model: State<'_, Arc<ModelService>>) -> bool {
    model.default_model_ready()
}

#[tauri::command]
pub fn model_setup_status(model: State<'_, Arc<ModelService>>) -> bool {
    model.model_catalog().iter().any(|m| model.model_downloaded(m.id))
}

#[tauri::command]
pub fn model_list(
    model: State<'_, Arc<ModelService>>,
    config: State<'_, Arc<ConfigService>>,
) -> Vec<ModelListItem> {
    let cfg = config.get();
    model
        .model_catalog()
        .iter()
        .map(|item| {
            let path = model.model_path_for_id(item.id);
            let selected = cfg.selected_model_id.as_deref() == Some(item.id)
                || cfg.scribe_model_path.as_ref().is_some_and(|p| {
                    path.as_ref()
                        .map(|mp| mp.to_string_lossy().as_ref() == p)
                        .unwrap_or(false)
                });
            ModelListItem {
                id: item.id.to_string(),
                label: item.label.to_string(),
                file_name: item.file_name.to_string(),
                downloaded: model.model_downloaded(item.id),
                selected,
            }
        })
        .collect()
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
    tauri::async_runtime::spawn(async move {
        if let Err(e) = m.download_default(&app).await {
            eprintln!("model download failed: {e}");
            app.emit("model://download-error", e.to_string()).ok();
        }
    });
    Ok(())
}

#[tauri::command]
pub fn model_download(
    model_id: String,
    model: State<'_, Arc<ModelService>>,
    app: AppHandle,
) -> Result<(), String> {
    let m = Arc::clone(&model);
    tauri::async_runtime::spawn(async move {
        if let Err(e) = m.download_model(&model_id, &app).await {
            eprintln!("model download failed: {e}");
            app.emit("model://download-error", e.to_string()).ok();
        }
    });
    Ok(())
}

#[tauri::command]
pub fn model_select(
    model_id: String,
    model: State<'_, Arc<ModelService>>,
    config: State<'_, Arc<ConfigService>>,
) -> Result<(), String> {
    let path = model
        .model_path_for_id(&model_id)
        .ok_or_else(|| format!("unknown model id: {model_id}"))?;
    if !model.model_available(&path) {
        return Err(format!("model {model_id} is not downloaded yet"));
    }
    let chosen_path = path.to_string_lossy().to_string();
    config
        .update(|cfg| {
            cfg.selected_model_id = Some(model_id.clone());
            cfg.scribe_model_path = Some(chosen_path);
        })
        .map_err(|e| e.to_string())
}
