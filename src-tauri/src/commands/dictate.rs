use crate::controllers::dictate::DictateController;
use crate::types::DictateHistoryEntry;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn dictate_cancel(
    ctrl: State<'_, Arc<DictateController>>,
    app: AppHandle,
) -> Result<(), String> {
    ctrl.cancel().map_err(|e| e.to_string())?;
    if let Some(w) = app.get_webview_window(crate::DICTATE_WINDOW_LABEL) {
        w.hide().map_err(|e| e.to_string())?;
        crate::platform::window_impl::sync_activation_policy(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn dictate_get_history(
    ctrl: State<'_, Arc<DictateController>>,
) -> Result<Vec<DictateHistoryEntry>, String> {
    ctrl.get_history()
}
