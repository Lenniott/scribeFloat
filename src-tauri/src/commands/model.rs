use crate::controllers::model::ModelController;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn model_vad_status(ctrl: State<'_, Arc<ModelController>>) -> bool {
    ctrl.vad_model_status()
}
