use crate::controllers::scribe::ScribeController;
use crate::types::Note;
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub fn scribe_start(
    ctrl: State<'_, Arc<ScribeController>>,
    preferred_mic: Option<String>,
    preferred_speaker: Option<String>,
    capture_speaker: bool,
) -> Result<(), String> {
    ctrl.start(preferred_mic, preferred_speaker, capture_speaker)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scribe_stop_and_save(
    ctrl: State<'_, Arc<ScribeController>>,
    title: Option<String>,
) -> Result<(), String> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || {
        ScribeController::stop_and_save(ctrl, title).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn scribe_save_recording_only(
    ctrl: State<'_, Arc<ScribeController>>,
    title: Option<String>,
) -> Result<(), String> {
    ctrl.save_recording_only(title)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_abort_transcription(ctrl: State<'_, Arc<ScribeController>>) -> Result<(), String> {
    ctrl.abort_transcription_keep_wav()
        .map_err(|e| e.to_string())
}

/// Hide the Scribe window without destroying it. Destroying the last window would quit the
/// tray-backed process; hide matches native close behaviour (`CloseRequested` → hide).
///
/// Always tries to end an active recording first so mic/speaker streams release even if the
/// frontend hid the window without awaiting `scribe_cancel` (focus/auto-start races).
#[tauri::command]
pub fn scribe_destroy_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(ctrl) = app.try_state::<Arc<ScribeController>>() {
        let _ = ctrl.cancel();
    }
    if let Some(w) = app.get_webview_window(crate::SCRIBE_WINDOW_LABEL) {
        w.hide().map_err(|e| e.to_string())?;
    }
    crate::platform::window_impl::sync_activation_policy(&app);
    Ok(())
}

#[tauri::command]
pub fn scribe_cancel(ctrl: State<'_, Arc<ScribeController>>) -> Result<(), String> {
    ctrl.cancel().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_add_note(
    ctrl: State<'_, Arc<ScribeController>>,
    text: String,
) -> Result<Note, String> {
    ctrl.add_note(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_get_include_timestamps(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<bool, String> {
    Ok(ctrl.get_include_timestamps())
}

#[tauri::command]
pub fn scribe_set_include_timestamps(
    ctrl: State<'_, Arc<ScribeController>>,
    enabled: bool,
) -> Result<(), String> {
    ctrl.set_include_timestamps(enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_list_input_devices(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<Vec<String>, String> {
    Ok(ctrl.list_input_devices())
}

#[tauri::command]
pub fn scribe_list_output_devices(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<Vec<String>, String> {
    Ok(ctrl.list_output_devices())
}

#[tauri::command]
pub async fn scribe_toggle_speaker_capture(
    ctrl: State<'_, Arc<ScribeController>>,
    enabled: bool,
) -> Result<(), String> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || {
        ctrl.toggle_speaker_capture(enabled).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn scribe_read_transcript(
    ctrl: State<'_, Arc<ScribeController>>,
    path: String,
) -> Result<String, String> {
    ctrl.read_transcript_at(&path)
}
