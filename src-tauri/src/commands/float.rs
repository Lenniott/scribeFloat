use crate::controllers::float::FloatController;
use crate::types::{AppError, FloatConfig, FloatModelInfo};
use std::sync::Arc;
use tauri::State;

/// Return the current Float configuration (provider, endpoint, model).
/// The API key is never included — only a `has_api_key` bool is returned.
#[tauri::command]
pub fn float_get_config(ctrl: State<'_, Arc<FloatController>>) -> Result<FloatConfig, AppError> {
    Ok(ctrl.get_config())
}

/// Persist Float provider settings.
/// Pass `api_key: null` to leave an existing key unchanged.
/// Pass `endpoint_url: ""` to reset to the provider's default URL.
#[tauri::command]
pub fn float_set_config(
    ctrl: State<'_, Arc<FloatController>>,
    provider: String,
    endpoint_url: String,
    api_key: Option<String>,
    model: Option<String>,
) -> Result<(), AppError> {
    ctrl.set_config(provider, endpoint_url, api_key, model)
        .map_err(AppError::from)
}

/// Clear the stored API key without changing other settings.
#[tauri::command]
pub fn float_clear_api_key(ctrl: State<'_, Arc<FloatController>>) -> Result<(), AppError> {
    ctrl.clear_api_key().map_err(AppError::from)
}

/// Fetch the list of models available from the configured provider.
/// Requires the provider to be reachable (Ollama running, or valid API key for cloud).
#[tauri::command]
pub async fn float_list_models(
    ctrl: State<'_, Arc<FloatController>>,
) -> Result<Vec<FloatModelInfo>, AppError> {
    let ctrl = Arc::clone(&ctrl);
    ctrl.list_models().await.map_err(AppError::from)
}

/// Probe the configured endpoint and return Ok if at least one model is reachable.
/// Use this to validate settings before saving — surface the error to the user.
#[tauri::command]
pub async fn float_test_connection(
    ctrl: State<'_, Arc<FloatController>>,
) -> Result<(), AppError> {
    let ctrl = Arc::clone(&ctrl);
    ctrl.test_connection().await.map_err(AppError::from)
}
