use crate::services::config::ConfigService;
use crate::services::permissions::PermissionsService;
use crate::types::PermissionStatus;
use std::path::Path;
use std::sync::Arc;

pub struct SettingsController {
    config: Arc<ConfigService>,
    permissions: Arc<PermissionsService>,
}

impl SettingsController {
    pub fn new(config: Arc<ConfigService>, permissions: Arc<PermissionsService>) -> Arc<Self> {
        Arc::new(Self {
            config,
            permissions,
        })
    }

    pub fn get_output_path(&self) -> String {
        self.config.get().save_folder
    }

    pub fn set_output_path(&self, path: String) -> Result<(), String> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("output path cannot be empty".to_string());
        }
        std::fs::create_dir_all(trimmed).map_err(|e| format!("failed to create output path: {e}"))?;
        if !Path::new(trimmed).is_dir() {
            return Err("output path must be a directory".to_string());
        }

        self.config
            .update(|cfg| cfg.save_folder = trimmed.to_string())
            .map_err(|e| format!("failed to persist output path: {e}"))
    }

    pub fn get_hotkeys(&self) -> (String, String) {
        let cfg = self.config.get();
        (cfg.open_scribe_hotkey, cfg.dictate_hotkey)
    }

    pub fn set_hotkeys(&self, open_scribe: String, dictate: String) -> Result<(), String> {
        let open_scribe = open_scribe.trim();
        let dictate = dictate.trim();
        if open_scribe.is_empty() || dictate.is_empty() {
            return Err("hotkeys cannot be empty".to_string());
        }
        if open_scribe.eq_ignore_ascii_case(dictate) {
            return Err("hotkeys cannot conflict".to_string());
        }

        self.config
            .update(|cfg| {
                cfg.open_scribe_hotkey = open_scribe.to_string();
                cfg.dictate_hotkey = dictate.to_string();
            })
            .map_err(|e| format!("failed to persist hotkeys: {e}"))
    }

    pub fn get_input_labels(&self) -> (String, String) {
        let cfg = self.config.get();
        (cfg.input_label, cfg.output_label)
    }

    pub fn set_input_labels(&self, input_label: String, output_label: String) -> Result<(), String> {
        let input_label = input_label.trim();
        let output_label = output_label.trim();
        if input_label.is_empty() || output_label.is_empty() {
            return Err("labels cannot be empty".to_string());
        }

        self.config
            .update(|cfg| {
                cfg.input_label = input_label.to_string();
                cfg.output_label = output_label.to_string();
            })
            .map_err(|e| format!("failed to persist labels: {e}"))
    }

    pub fn permission_statuses(&self) -> Vec<PermissionStatus> {
        self.permissions.statuses()
    }

    pub fn open_permission_settings(&self, kind: &str) -> Result<bool, String> {
        self.permissions
            .open_settings(kind)
            .map_err(|e| format!("failed to open permission settings: {e}"))
    }
}
