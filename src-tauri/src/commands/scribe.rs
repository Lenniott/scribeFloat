use crate::controllers::scribe::ScribeController;
use crate::types::{AppError, Note};
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub fn scribe_start(
    ctrl: State<'_, Arc<ScribeController>>,
    preferred_mic: Option<String>,
    preferred_speaker: Option<String>,
    capture_speaker: bool,
) -> Result<(), AppError> {
    ctrl.start(preferred_mic, preferred_speaker, capture_speaker)
        .map_err(AppError::from)
}

#[tauri::command]
pub async fn scribe_stop_and_save(
    ctrl: State<'_, Arc<ScribeController>>,
    title: Option<String>,
) -> Result<(), AppError> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || {
        ScribeController::stop_and_save(ctrl, title).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub fn scribe_save_recording_only(
    ctrl: State<'_, Arc<ScribeController>>,
    title: Option<String>,
) -> Result<(), AppError> {
    ctrl.save_recording_only(title)
        .map(|_| ())
        .map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_abort_transcription(ctrl: State<'_, Arc<ScribeController>>) -> Result<(), AppError> {
    ctrl.abort_transcription_keep_wav().map_err(AppError::from)
}

/// Hide the Scribe window without destroying it. Destroying the last window would quit the
/// tray-backed process; hide matches native close behaviour (`CloseRequested` → hide).
///
/// Always tries to end an active recording first so mic/speaker streams release even if the
/// frontend hid the window without awaiting `scribe_cancel`.
#[tauri::command]
pub fn scribe_destroy_window(app: tauri::AppHandle) -> Result<(), AppError> {
    if let Some(ctrl) = app.try_state::<Arc<ScribeController>>() {
        let _ = ctrl.cancel();
    }
    if let Some(w) = app.get_webview_window(crate::SCRIBE_WINDOW_LABEL) {
        w.hide().map_err(|e| AppError::Internal(e.to_string()))?;
    }
    crate::platform::window_impl::sync_activation_policy(&app);
    Ok(())
}

#[tauri::command]
pub fn scribe_cancel(ctrl: State<'_, Arc<ScribeController>>) -> Result<(), AppError> {
    ctrl.cancel().map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_add_note(
    ctrl: State<'_, Arc<ScribeController>>,
    text: String,
) -> Result<Note, AppError> {
    ctrl.add_note(text).map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_get_include_timestamps(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_include_timestamps())
}

#[tauri::command]
pub fn scribe_set_include_timestamps(
    ctrl: State<'_, Arc<ScribeController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_include_timestamps(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_list_input_devices(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<Vec<String>, AppError> {
    Ok(ctrl.list_input_devices())
}

#[tauri::command]
pub fn scribe_list_output_devices(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<Vec<String>, AppError> {
    Ok(ctrl.list_output_devices())
}

#[tauri::command]
pub async fn scribe_toggle_speaker_capture(
    ctrl: State<'_, Arc<ScribeController>>,
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
    ctrl: State<'_, Arc<ScribeController>>,
    path: String,
) -> Result<String, AppError> {
    ctrl.read_transcript_at(&path).map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_list_recovery_sessions(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<Vec<crate::types::RecoverySessionInfo>, AppError> {
    ctrl.list_recovery_sessions().map_err(AppError::from)
}

#[tauri::command]
pub fn scribe_list_transcripts(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<Vec<crate::types::ScribeTranscriptEntry>, AppError> {
    ctrl.list_transcripts().map_err(AppError::from)
}
