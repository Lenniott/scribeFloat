use crate::controllers::scribe::ScribeController;
use crate::types::{Note, ScribeStateEvent};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn scribe_start(
    ctrl: State<'_, Arc<ScribeController>>,
    preferred_mic: Option<String>,
) -> Result<(), String> {
    ctrl.start(preferred_mic).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_stop_and_save(
    ctrl: State<'_, Arc<ScribeController>>,
    title: Option<String>,
) -> Result<(), String> {
    ScribeController::stop_and_save(Arc::clone(&ctrl), title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_cancel(ctrl: State<'_, Arc<ScribeController>>) -> Result<(), String> {
    ctrl.cancel().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scribe_get_state(
    ctrl: State<'_, Arc<ScribeController>>,
) -> Result<ScribeStateEvent, String> {
    Ok(ctrl.get_state())
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
    ctrl.set_include_timestamps(enabled).map_err(|e| e.to_string())
}
