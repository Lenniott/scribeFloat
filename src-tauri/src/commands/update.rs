use crate::services::update::UpdateService;
use crate::types::{AppError, UpdateCheckResult};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn update_check(svc: State<'_, Arc<UpdateService>>) -> Result<UpdateCheckResult, AppError> {
    svc.check_for_update().await.map_err(AppError::from)
}
