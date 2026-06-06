use crate::controllers::dictate::DictateController;
use crate::types::{AppError, DictateHistoryEntry};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn dictate_cancel(ctrl: State<'_, Arc<DictateController>>) -> Result<(), AppError> {
    ctrl.cancel().map_err(AppError::from)
}

#[tauri::command]
pub fn dictate_dismiss(ctrl: State<'_, Arc<DictateController>>) {
    ctrl.dismiss();
}

#[tauri::command]
pub fn dictate_get_history(
    ctrl: State<'_, Arc<DictateController>>,
) -> Result<Vec<DictateHistoryEntry>, AppError> {
    ctrl.get_history().map_err(AppError::from)
}
