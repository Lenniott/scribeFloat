use crate::controllers::transcribe::{TranscribeController, TranscribeStartRequest};
use crate::types::TranscribeQueueItem;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn transcribe_inspect_inputs(
    ctrl: State<'_, Arc<TranscribeController>>,
    input_paths: Vec<String>,
) -> Result<Vec<TranscribeQueueItem>, String> {
    validate_input_paths(&input_paths)?;
    ctrl.inspect_inputs(input_paths)
}

#[tauri::command]
pub fn transcribe_start(
    ctrl: State<'_, Arc<TranscribeController>>,
    input_paths: Vec<String>,
    output_folder: Option<String>,
    model_id: Option<String>,
    include_timestamps: Option<bool>,
) -> Result<(), String> {
    validate_input_paths(&input_paths)?;
    let request = TranscribeStartRequest {
        input_paths,
        output_folder,
        model_id,
        include_timestamps,
    };
    TranscribeController::start(Arc::clone(&ctrl), request)
}

#[tauri::command]
pub fn transcribe_open_output(
    ctrl: State<'_, Arc<TranscribeController>>,
    file_path: String,
) -> Result<(), String> {
    let path = file_path.trim();
    if path.is_empty() {
        return Err("file_path cannot be empty".to_string());
    }
    ctrl.open_output_path(path)
}

#[tauri::command]
pub fn transcribe_show_window(app: AppHandle) -> Result<(), String> {
    crate::open_transcribe_window(&app)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn validate_input_paths(paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("at least one input path is required".to_string());
    }
    for path in paths {
        if path.trim().is_empty() {
            return Err("input path cannot be empty".to_string());
        }
    }
    Ok(())
}
