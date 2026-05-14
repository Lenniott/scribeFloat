use crate::controllers::settings::SettingsController;
use crate::types::{PermissionStatus, ReplacementRule, ThemeMode};
use std::sync::Arc;
use tauri::{AppHandle, State};

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
pub fn settings_get_preferred_audio_devices(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<(Option<String>, Option<String>), String> {
    Ok(ctrl.get_preferred_audio_devices())
}

#[tauri::command]
pub fn settings_set_preferred_audio_devices(
    ctrl: State<'_, Arc<SettingsController>>,
    preferred_input_device: Option<String>,
    preferred_speaker_device: Option<String>,
) -> Result<(), String> {
    ctrl.set_preferred_audio_devices(preferred_input_device, preferred_speaker_device)
}

#[tauri::command]
pub fn settings_list_output_devices(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Vec<String>, String> {
    Ok(ctrl.list_output_devices())
}

#[tauri::command]
pub fn settings_get_scribe_capture_speaker(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, String> {
    Ok(ctrl.get_scribe_capture_speaker())
}

#[tauri::command]
pub fn settings_set_scribe_capture_speaker(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), String> {
    ctrl.set_scribe_capture_speaker(enabled)
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

#[tauri::command]
pub fn settings_get_dictate_auto_paste(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, String> {
    Ok(ctrl.get_dictate_auto_paste())
}

#[tauri::command]
pub fn settings_set_dictate_auto_paste(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), String> {
    ctrl.set_dictate_auto_paste(enabled)
}

#[tauri::command]
pub fn settings_get_dictate_auto_enter(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, String> {
    Ok(ctrl.get_dictate_auto_enter())
}

#[tauri::command]
pub fn settings_set_dictate_auto_enter(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), String> {
    ctrl.set_dictate_auto_enter(enabled)
}

#[tauri::command]
pub fn settings_get_keep_wav(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<bool, String> {
    Ok(ctrl.get_keep_wav())
}

#[tauri::command]
pub fn settings_set_keep_wav(
    ctrl: State<'_, Arc<SettingsController>>,
    enabled: bool,
) -> Result<(), String> {
    ctrl.set_keep_wav(enabled)
}

#[tauri::command]
pub fn settings_get_dictate_model_id(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Option<String>, String> {
    Ok(ctrl.get_dictate_model_id())
}

#[tauri::command]
pub fn settings_set_dictate_model_id(
    ctrl: State<'_, Arc<SettingsController>>,
    model_id: Option<String>,
) -> Result<(), String> {
    ctrl.set_dictate_model_id(model_id)
}

#[tauri::command]
pub fn settings_show_window(app: AppHandle) -> Result<(), String> {
    crate::open_settings_window(&app)
        .map(|_| ())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn settings_get_replacement_rules(
    ctrl: State<'_, Arc<SettingsController>>,
) -> Result<Vec<ReplacementRule>, String> {
    Ok(ctrl.get_replacement_rules())
}

#[tauri::command]
pub fn settings_add_replacement_rule(
    ctrl: State<'_, Arc<SettingsController>>,
    rule: ReplacementRule,
) -> Result<(), String> {
    ctrl.add_replacement_rule(rule)
}

#[tauri::command]
pub fn settings_update_replacement_rule(
    ctrl: State<'_, Arc<SettingsController>>,
    index: usize,
    rule: ReplacementRule,
) -> Result<(), String> {
    ctrl.update_replacement_rule(index, rule)
}

#[tauri::command]
pub fn settings_delete_replacement_rule(
    ctrl: State<'_, Arc<SettingsController>>,
    index: usize,
) -> Result<(), String> {
    ctrl.delete_replacement_rule(index)
}
