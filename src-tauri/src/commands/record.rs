use crate::controllers::scribe::RecordController;
use crate::types::{AppError, Note};
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub fn scribe_start(
    ctrl: State<'_, Arc<RecordController>>,
    preferred_mic: Option<String>,
    preferred_speaker: Option<String>,
    capture_speaker: bool,
) -> Result<(), AppError> {
    ctrl.start(preferred_mic, preferred_speaker, capture_speaker)
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn scribe_stop_and_save(
    ctrl: State<'_, Arc<RecordController>>,
    title: Option<String>,
) -> Result<(), AppError> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || {
        RecordController::stop_and_save(ctrl, title).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub fn scribe_save_recording_only(
    ctrl: State<'_, Arc<RecordController>>,
    title: Option<String>,
) -> Result<(), AppError> {
    ctrl.save_recording_only(title)
        .map(|_| ())
        .map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_abort_transcription(ctrl: State<'_, Arc<RecordController>>) -> Result<(), AppError> {
    ctrl.abort_transcription_keep_wav().map_err(AppError::from)
}

/// No-op: Scribe lives in the main shell window; kept for legacy frontend callers.
#[tauri::command]
pub fn scribe_destroy_window(app: tauri::AppHandle) -> Result<(), AppError> {
    if let Some(ctrl) = app.try_state::<Arc<RecordController>>() {
        let _ = ctrl.cancel();
    }
    crate::platform::window_impl::sync_activation_policy(&app);
    Ok(())
}

#[tauri::command]
pub fn scribe_cancel(ctrl: State<'_, Arc<RecordController>>) -> Result<(), AppError> {
    ctrl.cancel().map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_add_note(
    ctrl: State<'_, Arc<RecordController>>,
    text: String,
) -> Result<Note, AppError> {
    ctrl.add_note(text).map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_get_include_timestamps(
    ctrl: State<'_, Arc<RecordController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_include_timestamps())
}

#[tauri::command]
pub fn scribe_set_include_timestamps(
    ctrl: State<'_, Arc<RecordController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_include_timestamps(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_list_input_devices(
    ctrl: State<'_, Arc<RecordController>>,
) -> Result<Vec<String>, AppError> {
    Ok(ctrl.list_input_devices())
}

#[tauri::command]
pub fn scribe_list_output_devices(
    ctrl: State<'_, Arc<RecordController>>,
) -> Result<Vec<String>, AppError> {
    Ok(ctrl.list_output_devices())
}

#[tauri::command]
pub async fn scribe_toggle_speaker_capture(
    ctrl: State<'_, Arc<RecordController>>,
    enabled: bool,
) -> Result<(), AppError> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || {
        ctrl.toggle_speaker_capture(enabled).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub fn scribe_read_transcript(
    ctrl: State<'_, Arc<RecordController>>,
    path: String,
) -> Result<String, AppError> {
    ctrl.read_transcript_at(&path).map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_list_recovery_sessions(
    ctrl: State<'_, Arc<RecordController>>,
) -> Result<Vec<crate::types::RecoverySessionInfo>, AppError> {
    ctrl.list_recovery_sessions().map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_list_transcripts(
    ctrl: State<'_, Arc<RecordController>>,
) -> Result<Vec<crate::types::ScribeTranscriptEntry>, AppError> {
    ctrl.list_transcripts().map_err(AppError::from)
}
