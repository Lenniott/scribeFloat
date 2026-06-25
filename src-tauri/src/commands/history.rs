use crate::controllers::history::HistoryController;
use crate::controllers::scribe::ScribeController;
use crate::types::{AppError, DashboardStats, HistoryListItem, HistoryRecord, TagVocabularyEntry};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

fn validate_id(id: &str) -> Result<(), AppError> {
    if id.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "history id must not be empty".to_string(),
        ));
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
pub fn history_delete(ctrl: State<'_, Arc<HistoryController>>, id: String) -> Result<(), AppError> {
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

#[tauri::command]
pub fn get_dashboard_stats(
    ctrl: State<'_, Arc<HistoryController>>,
) -> Result<DashboardStats, AppError> {
    ctrl.dashboard_stats().map_err(AppError::from)
}

#[tauri::command]
pub fn history_tag_vocabulary(
    ctrl: State<'_, Arc<HistoryController>>,
) -> Result<Vec<TagVocabularyEntry>, AppError> {
    ctrl.tag_vocabulary().map_err(AppError::from)
}

#[tauri::command]
pub fn note_create_empty(
    ctrl: State<'_, Arc<HistoryController>>,
    app: AppHandle,
) -> Result<String, AppError> {
    let id = ctrl.create_written_note().map_err(AppError::from)?;
    app.emit("note://item-added", ()).ok();
    Ok(id)
}

#[tauri::command]
pub fn note_save_written_content(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
    content: String,
) -> Result<(), AppError> {
    validate_id(&id)?;
    ctrl.save_written_content(&id, &content)
        .map_err(AppError::from)
}

#[tauri::command]
pub fn note_save_title(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
    title: String,
) -> Result<(), AppError> {
    validate_id(&id)?;
    ctrl.save_title(&id, &title).map_err(AppError::from)
}

#[tauri::command]
pub fn note_is_empty(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<bool, AppError> {
    validate_id(&id)?;
    ctrl.is_empty(&id).map_err(AppError::from)
}

#[tauri::command]
pub fn note_has_metadata(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<bool, AppError> {
    validate_id(&id)?;
    ctrl.has_metadata(&id).map_err(AppError::from)
}

#[tauri::command]
pub fn note_attach_transcript(
    history: State<'_, Arc<HistoryController>>,
    scribe: State<'_, Arc<ScribeController>>,
    app: AppHandle,
    id: String,
) -> Result<(), AppError> {
    validate_id(&id)?;
    let pending = scribe
        .take_pending_attach()
        .ok_or_else(|| AppError::InvalidInput("no transcript ready to attach".to_string()))?;
    history
        .attach_transcript(
            &id,
            pending.segments,
            pending.speaker_blocks,
            pending.notes,
            pending.model,
            pending.speaker_capture,
            pending.dual_source,
            pending.session_dir,
            pending.audio_path,
            pending.markdown_path,
        )
        .map_err(AppError::from)?;
    app.emit("note://item-added", ()).ok();
    Ok(())
}

#[tauri::command]
pub fn note_render_transcript_html(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<String, AppError> {
    validate_id(&id)?;
    ctrl.render_transcript_html(&id).map_err(AppError::from)
}
