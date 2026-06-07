use crate::controllers::history::HistoryController;
use crate::types::{AppError, HistoryListItem, HistoryRecord};
use std::sync::Arc;
use tauri::State;

fn validate_id(id: &str) -> Result<(), AppError> {
    if id.trim().is_empty() {
        return Err(AppError::InvalidInput("history id must not be empty".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_id_rejected() {
        let err = validate_id("").unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn whitespace_only_id_rejected() {
        let err = validate_id("   ").unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn valid_id_accepted() {
        assert!(validate_id("abc123").is_ok());
    }

    #[test]
    fn uuid_style_id_accepted() {
        assert!(validate_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }
}

#[tauri::command]
pub fn history_list(
    ctrl: State<'_, Arc<HistoryController>>,
) -> Result<Vec<HistoryListItem>, AppError> {
    ctrl.list().map_err(AppError::from)
}

#[tauri::command]
pub fn history_get_detail(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<HistoryRecord, AppError> {
    validate_id(&id)?;
    ctrl.get_detail(&id).map_err(AppError::from)
}

#[tauri::command]
pub fn history_render_markdown(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<String, AppError> {
    validate_id(&id)?;
    ctrl.render_markdown(&id).map_err(AppError::from)
}

#[tauri::command]
pub fn history_export_markdown(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<String, AppError> {
    validate_id(&id)?;
    ctrl.export_markdown(&id).map_err(AppError::from)
}

#[tauri::command]
pub fn history_delete(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<(), AppError> {
    validate_id(&id)?;
    ctrl.delete(&id).map_err(AppError::from)
}

#[tauri::command]
pub fn history_read_legacy(
    ctrl: State<'_, Arc<HistoryController>>,
    path: String,
) -> Result<String, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::InvalidInput("path must not be empty".to_string()));
    }
    ctrl.read_legacy(&path).map_err(AppError::from)
}
