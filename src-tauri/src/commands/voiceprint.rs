use crate::controllers::history::HistoryController;
use crate::controllers::voiceprint::VoiceprintController;
use crate::services::voice_learning::EvidenceGateReport;
use crate::types::{
    AppError, HistoryRecord, SessionCaptureStart, SessionCaptureStatus, SessionSpeaker,
    VoiceprintClipResult, VoiceprintModelStatus, VoiceprintProfileSummary,
};
use std::sync::Arc;
use tauri::{AppHandle, State};

fn find_session_speaker<'a>(
    record: &'a HistoryRecord,
    session_speaker_id: &str,
) -> Result<&'a SessionSpeaker, AppError> {
    record
        .session_speakers
        .iter()
        .find(|speaker| speaker.session_speaker_id == session_speaker_id)
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "session speaker `{session_speaker_id}` not found on this note"
            ))
        })
}

#[tauri::command]
pub fn voiceprint_evaluate_session_evidence(
    history: State<'_, Arc<HistoryController>>,
    ctrl: State<'_, Arc<VoiceprintController>>,
    note_id: String,
    session_speaker_id: String,
) -> Result<EvidenceGateReport, AppError> {
    let record = history.get_detail(&note_id).map_err(AppError::from)?;
    let speaker = find_session_speaker(&record, &session_speaker_id)?;
    ctrl.evaluate_session_evidence(speaker, &record.speaker_chunks)
        .map_err(AppError::from)
}

#[tauri::command]
pub fn voiceprint_apply_session_evidence(
    history: State<'_, Arc<HistoryController>>,
    ctrl: State<'_, Arc<VoiceprintController>>,
    note_id: String,
    session_speaker_id: String,
    profile_name: String,
) -> Result<(), AppError> {
    let record = history.get_detail(&note_id).map_err(AppError::from)?;
    let speaker = find_session_speaker(&record, &session_speaker_id)?;
    ctrl.apply_session_evidence(&note_id, speaker, &record.speaker_chunks, &profile_name)
        .map_err(AppError::from)
}

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
pub fn voiceprint_delete_all_profiles(
    ctrl: State<'_, Arc<VoiceprintController>>,
) -> Result<usize, AppError> {
    ctrl.delete_all_profiles().map_err(AppError::from)
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
