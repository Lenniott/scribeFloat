use crate::controllers::voiceprint::VoiceprintController;
use crate::types::{AppError, VoiceprintModelStatus, VoiceprintProfileSummary};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn voiceprint_list_profiles(
    ctrl: State<'_, Arc<VoiceprintController>>,
) -> Result<Vec<VoiceprintProfileSummary>, AppError> {
    ctrl.list_profiles().map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_list_profile_names(
    ctrl: State<'_, Arc<VoiceprintController>>,
) -> Result<Vec<String>, AppError> {
    ctrl.list_profile_names().map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_delete_profile(
    ctrl: State<'_, Arc<VoiceprintController>>,
    slug: String,
) -> Result<(), AppError> {
    ctrl.delete_profile(slug).map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_rename_profile(
    ctrl: State<'_, Arc<VoiceprintController>>,
    slug: String,
    name: String,
) -> Result<(), AppError> {
    ctrl.rename_profile(slug, name).map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_model_status(
    ctrl: State<'_, Arc<VoiceprintController>>,
) -> VoiceprintModelStatus {
    ctrl.model_status()
}

#[tauri::command]
pub fn voiceprint_download_model(
    ctrl: State<'_, Arc<VoiceprintController>>,
    app: AppHandle,
) -> Result<(), AppError> {
    Arc::clone(&ctrl)
        .download_model(app)
        .map_err(AppError::from)
}
