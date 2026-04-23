use crate::controllers::model::ModelController;
use crate::types::ModelListItem;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn model_setup_status(ctrl: State<'_, Arc<ModelController>>) -> bool {
    ctrl.setup_status()
}

#[tauri::command]
pub fn model_list(ctrl: State<'_, Arc<ModelController>>) -> Vec<ModelListItem> {
    ctrl.list_models()
}

#[tauri::command]
pub fn model_download(
    model_id: String,
    ctrl: State<'_, Arc<ModelController>>,
    app: AppHandle,
) -> Result<(), String> {
    Arc::clone(&ctrl).download_model(model_id, app);
    Ok(())
}

#[tauri::command]
pub fn model_select(model_id: String, ctrl: State<'_, Arc<ModelController>>) -> Result<(), String> {
    ctrl.select_model(model_id)
}
