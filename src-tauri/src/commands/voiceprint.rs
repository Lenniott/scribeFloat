use crate::controllers::voiceprint::VoiceprintController;
use crate::types::{
    AppError, SessionCaptureStart, SessionCaptureStatus, VoiceprintClipResult,
    VoiceprintModelStatus, VoiceprintProfileSummary,
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
pub async fn voiceprint_stop_clip(
    ctrl: State<'_, Arc<VoiceprintController>>,
    clip_id: String,
) -> Result<VoiceprintClipResult, AppError> {
    let ctrl = Arc::clone(&ctrl);
    tauri::async_runtime::spawn_blocking(move || ctrl.stop_clip(clip_id))
        .await
        .map_err(|e| AppError::from(e.to_string()))?
        .map_err(AppError::from)
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

#[tauri::command]
pub fn session_capture_start(
    ctrl: State<'_, Arc<VoiceprintController>>,
    app: AppHandle,
) -> Result<SessionCaptureStart, AppError> {
    ctrl.start_session_capture(app)
        .map(|capture_id| SessionCaptureStart { capture_id })
        .map_err(AppError::from)
}

#[tauri::command]
pub fn session_capture_status(
    ctrl: State<'_, Arc<VoiceprintController>>,
    capture_id: String,
) -> Result<SessionCaptureStatus, AppError> {
    ctrl.clip_status(capture_id)
        .map(|status| SessionCaptureStatus {
            capture_id: status.clip_id,
            speech_s: status.speech_s,
            purity: status.purity,
            state: status.state,
        })
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn session_capture_stop(
    ctrl: State<'_, Arc<VoiceprintController>>,
    capture_id: String,
    profile_name: String,
) -> Result<VoiceprintClipResult, AppError> {
    let ctrl = Arc::clone(&ctrl);
    tauri::async_runtime::spawn_blocking(move || -> Result<VoiceprintClipResult, String> {
        let result = ctrl.stop_clip(capture_id.clone())?;
        if result.accepted {
            ctrl.commit_clip(capture_id, profile_name)?;
        }
        Ok(result)
    })
    .await
    .map_err(|e| AppError::from(e.to_string()))?
    .map_err(AppError::from)
}

#[tauri::command]
pub fn session_capture_cancel(
    ctrl: State<'_, Arc<VoiceprintController>>,
    capture_id: String,
) -> Result<(), AppError> {
    ctrl.discard_clip(capture_id).map_err(AppError::from)
}
