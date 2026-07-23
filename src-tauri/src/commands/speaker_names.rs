use crate::controllers::speaker_names::SpeakerNamesController;
use crate::services::speaker_names::SpeakerName;
use crate::types::AppError;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn speaker_names_list(ctrl: State<'_, Arc<SpeakerNamesController>>) -> Vec<SpeakerName> {
    ctrl.list()
}

#[tauri::command]
pub fn speaker_name_save(
    ctrl: State<'_, Arc<SpeakerNamesController>>,
    name: String,
    previous_slug: Option<String>,
) -> Result<SpeakerName, AppError> {
    ctrl.save(&name, previous_slug.as_deref())
        .map_err(AppError::from)
}

#[tauri::command]
pub fn speaker_name_delete(
    ctrl: State<'_, Arc<SpeakerNamesController>>,
    slug: String,
) -> Result<bool, AppError> {
    ctrl.delete(&slug).map_err(AppError::from)
}
