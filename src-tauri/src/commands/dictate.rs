use crate::controllers::dictate::DictateController;
use crate::types::DictateHistoryEntry;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn dictate_cancel(ctrl: State<'_, Arc<DictateController>>) -> Result<(), String> {
    ctrl.cancel().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dictate_dismiss(ctrl: State<'_, Arc<DictateController>>) {
    ctrl.dismiss();
}

#[tauri::command]
pub fn dictate_get_history(
    ctrl: State<'_, Arc<DictateController>>,
) -> Result<Vec<DictateHistoryEntry>, String> {
    ctrl.get_history()
}
