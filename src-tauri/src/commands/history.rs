use crate::controllers::history::HistoryController;
use crate::types::{HistoryListItem, HistoryRecord};
use std::sync::Arc;
use tauri::State;

/// Reject empty/whitespace ids early with a descriptive error.
fn validate_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("history id must not be empty".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn history_list(
    ctrl: State<'_, Arc<HistoryController>>,
) -> Result<Vec<HistoryListItem>, String> {
    ctrl.list()
}

#[tauri::command]
pub fn history_get_detail(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<HistoryRecord, String> {
    validate_id(&id)?;
    ctrl.get_detail(&id)
}

#[tauri::command]
pub fn history_render_markdown(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<String, String> {
    validate_id(&id)?;
    ctrl.render_markdown(&id)
}

#[tauri::command]
pub fn history_export_markdown(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<String, String> {
    validate_id(&id)?;
    ctrl.export_markdown(&id)
}

#[tauri::command]
pub fn history_delete(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<(), String> {
    validate_id(&id)?;
    ctrl.delete(&id)
}

#[tauri::command]
pub fn history_read_legacy(
    ctrl: State<'_, Arc<HistoryController>>,
    path: String,
) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("path must not be empty".to_string());
    }
    ctrl.read_legacy(&path)
}
