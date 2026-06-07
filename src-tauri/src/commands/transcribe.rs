use crate::controllers::transcribe::{TranscribeController, TranscribeStartRequest};
use crate::types::{AppError, TranscribeQueueItem};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn transcribe_inspect_inputs(
    ctrl: State<'_, Arc<TranscribeController>>,
    input_paths: Vec<String>,
) -> Result<Vec<TranscribeQueueItem>, AppError> {
    validate_input_paths(&input_paths)?;
    ctrl.inspect_inputs(input_paths).map_err(AppError::from)
}

#[tauri::command]
pub fn transcribe_start(
    ctrl: State<'_, Arc<TranscribeController>>,
    input_paths: Vec<String>,
    output_folder: Option<String>,
    model_id: Option<String>,
    include_timestamps: Option<bool>,
) -> Result<(), AppError> {
    validate_input_paths(&input_paths)?;
    let request = TranscribeStartRequest {
        input_paths,
        output_folder,
        model_id,
        include_timestamps,
    };
    TranscribeController::start(Arc::clone(&ctrl), request).map_err(AppError::from)
}

#[tauri::command]
pub fn transcribe_open_output(
    ctrl: State<'_, Arc<TranscribeController>>,
    file_path: String,
) -> Result<(), AppError> {
    let path = file_path.trim();
    if path.is_empty() {
        return Err(AppError::InvalidInput("file_path cannot be empty".to_string()));
    }
    ctrl.open_output_path(path).map_err(AppError::from)
}

#[tauri::command]
pub fn transcribe_show_window(app: AppHandle) -> Result<(), AppError> {
    crate::open_transcribe_window(&app)
        .map(|_| ())
        .map_err(|e| AppError::Internal(e.to_string()))
}

fn validate_input_paths(paths: &[String]) -> Result<(), AppError> {
    if paths.is_empty() {
        return Err(AppError::InvalidInput("at least one input path is required".to_string()));
    }
    for path in paths {
        if path.trim().is_empty() {
            return Err(AppError::InvalidInput("input path cannot be empty".to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_paths_rejected() {
        let err = validate_input_paths(&[]).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn validate_blank_path_rejected() {
        let paths = vec!["  ".to_string()];
        let err = validate_input_paths(&paths).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn validate_nonempty_paths_accepted() {
        let paths = vec!["/some/file.wav".to_string()];
        assert!(validate_input_paths(&paths).is_ok());
    }

    #[test]
    fn validate_mixed_rejects_on_blank_entry() {
        let paths = vec!["/valid/path.wav".to_string(), "".to_string()];
        let err = validate_input_paths(&paths).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }
}
