use crate::controllers::voiceprint::VoiceprintController;
use crate::types::{
    AppError, VoiceprintClipResult, VoiceprintModelStatus, VoiceprintProfileSummary,
};
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

#[tauri::command]
pub fn voiceprint_start_clip(
    ctrl: State<'_, Arc<VoiceprintController>>,
    app: AppHandle,
    mic_device_id: String,
) -> Result<String, AppError> {
    ctrl.start_clip(mic_device_id, app).map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_stop_clip(
    ctrl: State<'_, Arc<VoiceprintController>>,
    clip_id: String,
) -> Result<VoiceprintClipResult, AppError> {
    ctrl.stop_clip(clip_id).map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_commit_clip(
    ctrl: State<'_, Arc<VoiceprintController>>,
    clip_id: String,
    profile_name: String,
) -> Result<(), AppError> {
    ctrl.commit_clip(clip_id, profile_name)
        .map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_discard_clip(
    ctrl: State<'_, Arc<VoiceprintController>>,
    clip_id: String,
) -> Result<(), AppError> {
    ctrl.discard_clip(clip_id).map_err(AppError::from)
}
