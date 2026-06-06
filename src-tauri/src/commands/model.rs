use crate::controllers::model::ModelController;
use crate::types::{AppError, ModelListItem};
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
) -> Result<(), AppError> {
    Arc::clone(&ctrl).download_model(model_id, app).map_err(AppError::from)
}

#[tauri::command]
pub fn model_select(model_id: String, ctrl: State<'_, Arc<ModelController>>) -> Result<(), AppError> {
    ctrl.select_model(model_id).map_err(AppError::from)
}

#[tauri::command]
pub fn model_remove(model_id: String, ctrl: State<'_, Arc<ModelController>>) -> Result<(), AppError> {
    ctrl.remove_model(model_id).map_err(AppError::from)
}

#[tauri::command]
pub fn model_vad_status(ctrl: State<'_, Arc<ModelController>>) -> bool {
    ctrl.vad_model_status()
}

#[tauri::command]
pub fn model_vad_download(
    ctrl: State<'_, Arc<ModelController>>,
    app: AppHandle,
) -> Result<(), AppError> {
    Arc::clone(&ctrl).download_vad_model(app).map_err(AppError::from)
}

#[tauri::command]
pub fn model_vad_remove(ctrl: State<'_, Arc<ModelController>>) -> Result<(), AppError> {
    ctrl.remove_vad_model().map_err(AppError::from)
}
