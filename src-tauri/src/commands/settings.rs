use crate::controllers::settings::SettingsController;
use crate::types::{AppError, PermissionStatus, ReplacementRule, ThemeMode};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn settings_get_output_path(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<String, AppError> {
    Ok(ctrl.get_output_path())
}

#[tauri::command]
pub fn settings_set_output_path(
    ctrl: State<'_, Arc<SettingsController>>,
    path: String,
) -> Result<(), AppError> {
    ctrl.set_output_path(path).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_hotkeys(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(String, String), AppError> {
    Ok(ctrl.get_hotkeys())
}

#[tauri::command]
pub fn settings_set_hotkeys(
    ctrl: State<'_, Arc<SettingsController>>,
    open_scribe: String,
    dictate: String,
) -> Result<(), AppError> {
    ctrl.set_hotkeys(open_scribe, dictate).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_input_labels(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(String, String), AppError> {
    Ok(ctrl.get_input_labels())
}

#[tauri::command]
pub fn settings_set_input_labels(
    ctrl: State<'_, Arc<SettingsController>>,
    input_label: String,
    output_label: String,
) -> Result<(), AppError> {
    ctrl.set_input_labels(input_label, output_label).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_preferred_audio_devices(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(Option<String>, Option<String>), AppError> {
    Ok(ctrl.get_preferred_audio_devices())
}

#[tauri::command]
pub fn settings_set_preferred_audio_devices(
    ctrl: State<'_, Arc<SettingsController>>,
    preferred_input_device: Option<String>,
    preferred_speaker_device: Option<String>,
) -> Result<(), AppError> {
    ctrl.set_preferred_audio_devices(preferred_input_device, preferred_speaker_device)
        .map_err(AppError::from)
}

#[tauri::command]
pub fn settings_list_output_devices(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Vec<String>, AppError> {
    Ok(ctrl.list_output_devices())
}

#[tauri::command]
pub fn settings_speaker_capture_requires_device_name() -> bool {
    SettingsController::speaker_capture_requires_device_name()
}

#[tauri::command]
pub fn settings_blackhole_detected(ctrl: State<'_, Arc<SettingsController>>) -> bool {
    ctrl.blackhole_device_detected()
}

#[tauri::command]
pub fn settings_get_scribe_capture_speaker(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_scribe_capture_speaker())
}

#[tauri::command]
pub fn settings_set_scribe_capture_speaker(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_scribe_capture_speaker(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_open_with_app_path(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Option<String>, AppError> {
    Ok(ctrl.get_open_with_app_path())
}

#[tauri::command]
pub fn settings_set_open_with_app_path(
    ctrl: State<'_, Arc<SettingsController>>,
    path: Option<String>,
) -> Result<(), AppError> {
    ctrl.set_open_with_app_path(path).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_open_transcript(
    ctrl: State<'_, Arc<SettingsController>>,
    file_path: String,
) -> Result<(), AppError> {
    ctrl.open_transcript(&file_path).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_theme_mode(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<ThemeMode, AppError> {
    Ok(ctrl.get_theme_mode())
}

#[tauri::command]
pub fn settings_set_theme_mode(
    ctrl: State<'_, Arc<SettingsController>>,
    theme_mode: String,
) -> Result<(), AppError> {
    ctrl.set_theme_mode(theme_mode).map_err(AppError::from)
}

#[tauri::command]
pub async fn settings_permissions_status(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Vec<PermissionStatus>, AppError> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || ctrl.permission_statuses())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn settings_permissions_open(
    ctrl: State<'_, Arc<SettingsController>>,
    kind: String,
) -> Result<bool, AppError> {
    ctrl.open_permission_settings(&kind).map_err(AppError::from)
}

#[tauri::command]
pub async fn settings_permissions_request(
    ctrl: State<'_, Arc<SettingsController>>,
    kind: String,
) -> Result<(), AppError> {
    let ctrl = Arc::clone(&ctrl);
    tokio::task::spawn_blocking(move || ctrl.request_permission(&kind))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map_err(AppError::from)
}

#[tauri::command]
pub fn settings_onboarding_status(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.is_onboarding_complete())
}

#[tauri::command]
pub fn settings_complete_onboarding(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(), AppError> {
    ctrl.complete_onboarding().map_err(AppError::from)
}

#[tauri::command]
pub fn settings_reset_onboarding(ctrl: State<'_, Arc<SettingsController>>) -> Result<(), AppError> {
    ctrl.reset_onboarding().map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_dictate_auto_paste(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_dictate_auto_paste())
}

#[tauri::command]
pub fn settings_set_dictate_auto_paste(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_dictate_auto_paste(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_dictate_auto_enter(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_dictate_auto_enter())
}

#[tauri::command]
pub fn settings_set_dictate_auto_enter(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_dictate_auto_enter(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_keep_wav(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_keep_wav())
}

#[tauri::command]
pub fn settings_set_keep_wav(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_keep_wav(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_save_transcripts_as_markdown(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, AppError> {
    Ok(ctrl.get_save_transcripts_as_markdown())
}

#[tauri::command]
pub fn settings_set_save_transcripts_as_markdown(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), AppError> {
    ctrl.set_save_transcripts_as_markdown(enabled).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_dictate_model_id(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Option<String>, AppError> {
    Ok(ctrl.get_dictate_model_id())
}

#[tauri::command]
pub fn settings_set_dictate_model_id(
    ctrl: State<'_, Arc<SettingsController>>,
    model_id: Option<String>,
) -> Result<(), AppError> {
    ctrl.set_dictate_model_id(model_id).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_show_window(app: AppHandle) -> Result<(), AppError> {
    crate::open_settings_window(&app)
        .map(|_| ())
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn settings_show_onboarding_window(app: AppHandle) -> Result<(), AppError> {
    crate::open_onboarding_window(&app)
        .map(|_| ())
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn settings_get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
pub fn settings_open_scribe_window(app: AppHandle) -> Result<(), AppError> {
    crate::open_scribe_window(&app)
        .map(|_| ())
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub fn settings_get_replacement_rules(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Vec<ReplacementRule>, AppError> {
    Ok(ctrl.get_replacement_rules())
}

#[tauri::command]
pub fn settings_add_replacement_rule(
    ctrl: State<'_, Arc<SettingsController>>,
    rule: ReplacementRule,
) -> Result<(), AppError> {
    ctrl.add_replacement_rule(rule).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_update_replacement_rule(
    ctrl: State<'_, Arc<SettingsController>>,
    index: usize,
    rule: ReplacementRule,
) -> Result<(), AppError> {
    ctrl.update_replacement_rule(index, rule).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_delete_replacement_rule(
    ctrl: State<'_, Arc<SettingsController>>,
    index: usize,
) -> Result<(), AppError> {
    ctrl.delete_replacement_rule(index).map_err(AppError::from)
}

#[tauri::command]
pub fn settings_get_replacement_prefix(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<String, String> {
    Ok(ctrl.get_replacement_prefix())
}

#[tauri::command]
pub fn settings_set_replacement_prefix(
    ctrl: State<'_, Arc<SettingsController>>,
    prefix: String,
) -> Result<(), String> {
    ctrl.set_replacement_prefix(prefix)
}
