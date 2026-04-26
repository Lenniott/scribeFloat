use crate::controllers::settings::SettingsController;
use crate::types::{PermissionStatus, ThemeMode};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn settings_get_output_path(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<String, String> {
    Ok(ctrl.get_output_path())
}

#[tauri::command]
pub fn settings_set_output_path(
    ctrl: State<'_, Arc<SettingsController>>,
    path: String,
) -> Result<(), String> {
    ctrl.set_output_path(path)
}

#[tauri::command]
pub fn settings_get_hotkeys(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(String, String), String> {
    Ok(ctrl.get_hotkeys())
}

#[tauri::command]
pub fn settings_set_hotkeys(
    ctrl: State<'_, Arc<SettingsController>>,
    open_scribe: String,
    dictate: String,
) -> Result<(), String> {
    ctrl.set_hotkeys(open_scribe, dictate)
}

#[tauri::command]
pub fn settings_get_input_labels(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(String, String), String> {
    Ok(ctrl.get_input_labels())
}

#[tauri::command]
pub fn settings_set_input_labels(
    ctrl: State<'_, Arc<SettingsController>>,
    input_label: String,
    output_label: String,
) -> Result<(), String> {
    ctrl.set_input_labels(input_label, output_label)
}

#[tauri::command]
pub fn settings_get_open_with_app_path(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Option<String>, String> {
    Ok(ctrl.get_open_with_app_path())
}

#[tauri::command]
pub fn settings_set_open_with_app_path(
    ctrl: State<'_, Arc<SettingsController>>,
    path: Option<String>,
) -> Result<(), String> {
    ctrl.set_open_with_app_path(path)
}

#[tauri::command]
pub fn settings_open_transcript(
    ctrl: State<'_, Arc<SettingsController>>,
    file_path: String,
) -> Result<(), String> {
    ctrl.open_transcript(&file_path)
}

#[tauri::command]
pub fn settings_get_theme_mode(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<ThemeMode, String> {
    Ok(ctrl.get_theme_mode())
}

#[tauri::command]
pub fn settings_set_theme_mode(
    ctrl: State<'_, Arc<SettingsController>>,
    theme_mode: String,
) -> Result<(), String> {
    ctrl.set_theme_mode(theme_mode)
}

#[tauri::command]
pub fn settings_permissions_status(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Vec<PermissionStatus>, String> {
    Ok(ctrl.permission_statuses())
}

#[tauri::command]
pub fn settings_permissions_open(
    ctrl: State<'_, Arc<SettingsController>>,
    kind: String,
) -> Result<bool, String> {
    ctrl.open_permission_settings(&kind)
}

#[tauri::command]
pub fn settings_permissions_request(
    ctrl: State<'_, Arc<SettingsController>>,
    kind: String,
) -> Result<(), String> {
    ctrl.request_permission(&kind)
}

#[tauri::command]
pub fn settings_onboarding_status(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, String> {
    Ok(ctrl.is_onboarding_complete())
}

#[tauri::command]
pub fn settings_complete_onboarding(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(), String> {
    ctrl.complete_onboarding()
}

#[tauri::command]
pub fn settings_reset_onboarding(ctrl: State<'_, Arc<SettingsController>>) -> Result<(), String> {
    ctrl.reset_onboarding()
}
