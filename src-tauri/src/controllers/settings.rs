use crate::services::config::ConfigService;
use crate::services::hotkeys::HotkeyService;
use crate::services::output::OutputService;
use crate::services::permissions::PermissionsService;
use crate::services::audio::AudioService;
use crate::types::{PermissionStatus, ThemeMode};
use std::path::Path;
use std::sync::Arc;

pub struct SettingsController {
    config: Arc<ConfigService>,
    hotkeys: Arc<HotkeyService>,
    output: Arc<OutputService>,
    permissions: Arc<PermissionsService>,
    audio: Arc<AudioService>,
}

impl SettingsController {
    pub fn new(
        config: Arc<ConfigService>,
        hotkeys: Arc<HotkeyService>,
        output: Arc<OutputService>,
        permissions: Arc<PermissionsService>,
        audio: Arc<AudioService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            hotkeys,
            output,
            permissions,
            audio,
        })
    }

    pub fn get_output_path(&self) -> String {
        self.config.get().save_folder
    }

    pub fn set_output_path(&self, path: String) -> Result<(), String> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("output path cannot be empty.".to_string());
        }

        let candidate = Path::new(trimmed);
        if !candidate.is_absolute() {
            return Err(format!("output path `{trimmed}` must be an absolute path."));
        }

        if candidate.exists() && !candidate.is_dir() {
            return Err(format!(
                "output path `{trimmed}` points to a file; expected a directory."
            ));
        }

        let normalized = self
            .output
            .ensure_output_dir(candidate)
            .map_err(|e| format!("output path `{trimmed}`: {e}"))?;
        let normalized = normalized.to_string_lossy().to_string();
        self.config
            .update(|cfg| cfg.save_folder = normalized.clone())
            .map_err(|e| format!("failed to persist output path: {e}"))
    }

    pub fn get_hotkeys(&self) -> (String, String) {
        let cfg = self.config.get();
        (cfg.open_scribe_hotkey, cfg.dictate_hotkey)
    }

    pub fn set_hotkeys(&self, open_scribe: String, dictate: String) -> Result<(), String> {
        let previous = self.get_hotkeys();
        let (open_scribe, dictate) = self.hotkeys.validate_pair(&open_scribe, &dictate)?;
        self.hotkeys.rebind(&open_scribe, &dictate)?;

        let persist_result = self
            .config
            .update(|cfg| {
                cfg.open_scribe_hotkey = open_scribe.clone();
                cfg.dictate_hotkey = dictate.clone();
            })
            .map_err(|e| format!("failed to persist hotkeys: {e}"));

        if persist_result.is_err() {
            let _ = self.hotkeys.rebind(&previous.0, &previous.1);
        }

        persist_result
    }

    pub fn rehydrate_hotkeys(&self) -> Result<(), String> {
        let (existing_open, existing_dictate) = self.get_hotkeys();
        let defaults = crate::types::Config::default();
        let validated = self
            .hotkeys
            .validate_pair(&existing_open, &existing_dictate)
            .or_else(|_| {
                self.hotkeys
                    .validate_pair(&defaults.open_scribe_hotkey, &defaults.dictate_hotkey)
            })?;

        self.hotkeys.rebind(&validated.0, &validated.1)?;

        if validated.0 != existing_open || validated.1 != existing_dictate {
            self.config
                .update(|cfg| {
                    cfg.open_scribe_hotkey = validated.0.clone();
                    cfg.dictate_hotkey = validated.1.clone();
                })
                .map_err(|e| format!("failed to persist normalized startup hotkeys: {e}"))?;
        }
        Ok(())
    }

    pub fn get_input_labels(&self) -> (String, String) {
        let cfg = self.config.get();
        (cfg.input_label, cfg.output_label)
    }

    pub fn set_input_labels(
        &self,
        input_label: String,
        output_label: String,
    ) -> Result<(), String> {
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

    pub fn get_preferred_audio_devices(&self) -> (Option<String>, Option<String>) {
        let cfg = self.config.get();
        (cfg.preferred_input_device, cfg.preferred_speaker_device)
    }

    pub fn set_preferred_audio_devices(
        &self,
        preferred_input_device: Option<String>,
        preferred_speaker_device: Option<String>,
    ) -> Result<(), String> {
        let normalize = |value: Option<String>| -> Option<String> {
            value.and_then(|v| {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
        };

        let preferred_input_device = normalize(preferred_input_device);
        let preferred_speaker_device = normalize(preferred_speaker_device);

        self.config
            .update(|cfg| {
                cfg.preferred_input_device = preferred_input_device.clone();
                cfg.preferred_speaker_device = preferred_speaker_device.clone();
            })
            .map_err(|e| format!("failed to persist preferred audio devices: {e}"))
    }

    pub fn list_output_devices(&self) -> Vec<String> {
        self.audio.list_output_devices()
    }

    pub fn get_scribe_capture_speaker(&self) -> bool {
        self.config.get().scribe_capture_speaker
    }

    pub fn set_scribe_capture_speaker(&self, enabled: bool) -> Result<(), String> {
        self.config
            .update(|cfg| cfg.scribe_capture_speaker = enabled)
            .map_err(|e| format!("failed to persist scribe speaker capture setting: {e}"))
    }

    pub fn get_open_with_app_path(&self) -> Option<String> {
        self.config.get().open_with_app_path
    }

    pub fn set_open_with_app_path(&self, path: Option<String>) -> Result<(), String> {
        if let Some(ref p) = path {
            let trimmed = p.trim();
            if trimmed.is_empty() {
                return Err("app path cannot be empty; pass null to clear it".to_string());
            }
            let candidate = std::path::Path::new(trimmed);
            if !candidate.is_absolute() {
                return Err("app path must be an absolute path".to_string());
            }
            if !candidate.exists() {
                return Err(format!("app path `{trimmed}` does not exist on this system"));
            }
        }
        self.config
            .update(|cfg| cfg.open_with_app_path = path)
            .map_err(|e| format!("failed to persist app path: {e}"))
    }

    pub fn open_transcript(&self, file_path: &str) -> Result<(), String> {
        let path = std::path::Path::new(file_path);
        let canonical = path
            .canonicalize()
            .map_err(|_| "invalid or inaccessible transcript path".to_string())?;
        let save_folder = self.config.get().save_folder;
        let base = std::path::Path::new(&save_folder)
            .canonicalize()
            .map_err(|_| "save folder is not accessible".to_string())?;
        if !canonical.starts_with(&base) {
            return Err("transcript path is outside the configured save folder".to_string());
        }
        let app = self.config.get().open_with_app_path;
        self.output
            .open_file_for_user(canonical.to_str().unwrap_or(file_path), app.as_deref())
    }

    pub fn get_theme_mode(&self) -> ThemeMode {
        self.config.get().theme_mode
    }

    pub fn set_theme_mode(&self, theme_mode: String) -> Result<(), String> {
        let theme_mode = ThemeMode::parse(&theme_mode)?;
        self.config
            .update(|cfg| cfg.theme_mode = theme_mode)
            .map_err(|e| format!("failed to persist theme mode: {e}"))
    }

    pub fn permission_statuses(&self) -> Vec<PermissionStatus> {
        self.permissions.statuses()
    }

    pub fn open_permission_settings(&self, kind: &str) -> Result<bool, String> {
        self.permissions
            .open_settings(kind)
            .map_err(|e| format!("failed to open permission settings: {e}"))
    }

    pub fn request_permission(&self, kind: &str) -> Result<(), String> {
        self.permissions
            .request_permission(kind)
            .map_err(|e| format!("failed to request permission for {kind}: {e}"))
    }

    pub fn get_dictate_auto_paste(&self) -> bool {
        self.config.get().dictate_auto_paste
    }

    pub fn set_dictate_auto_paste(&self, enabled: bool) -> Result<(), String> {
        self.config
            .update(|cfg| cfg.dictate_auto_paste = enabled)
            .map_err(|e| format!("failed to persist dictate_auto_paste: {e}"))
    }

    pub fn get_dictate_auto_enter(&self) -> bool {
        self.config.get().dictate_auto_enter
    }

    pub fn set_dictate_auto_enter(&self, enabled: bool) -> Result<(), String> {
        self.config
            .update(|cfg| cfg.dictate_auto_enter = enabled)
            .map_err(|e| format!("failed to persist dictate_auto_enter: {e}"))
    }

    pub fn get_dictate_model_id(&self) -> Option<String> {
        self.config.get().dictate_model_id
    }

    pub fn set_dictate_model_id(&self, model_id: Option<String>) -> Result<(), String> {
        let model_id = model_id.and_then(|id| {
            let trimmed = id.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        });
        self.config
            .update(|cfg| cfg.dictate_model_id = model_id)
            .map_err(|e| format!("failed to persist dictate_model_id: {e}"))
    }

    pub fn is_onboarding_complete(&self) -> bool {
        self.config.get().onboarding_complete
    }

    pub fn complete_onboarding(&self) -> Result<(), String> {
        self.config
            .update(|cfg| cfg.onboarding_complete = true)
            .map_err(|e| format!("failed to save onboarding status: {e}"))
    }

    pub fn reset_onboarding(&self) -> Result<(), String> {
        self.config
            .update(|cfg| cfg.onboarding_complete = false)
            .map_err(|e| format!("failed to reset onboarding status: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::hotkeys::HotkeyRegistrar;
    use std::sync::Mutex;

    struct MockHotkeyRegistrar {
        bindings: Mutex<Vec<(String, String)>>,
        fail_on_open: Option<String>,
    }

    impl HotkeyRegistrar for MockHotkeyRegistrar {
        fn rebind(&self, open_scribe: &str, dictate: &str) -> Result<(), String> {
            if self
                .fail_on_open
                .as_ref()
                .is_some_and(|expected| expected == open_scribe)
            {
                return Err("mock register failure".to_string());
            }
            self.bindings
                .lock()
                .unwrap()
                .push((open_scribe.to_string(), dictate.to_string()));
            Ok(())
        }
    }

    fn make_controller(
        config: Arc<ConfigService>,
        fail_on_open: Option<String>,
    ) -> Arc<SettingsController> {
        let registrar = Arc::new(MockHotkeyRegistrar {
            bindings: Mutex::new(Vec::new()),
            fail_on_open,
        });
        let hotkeys = HotkeyService::new(registrar);
        SettingsController::new(
            config,
            hotkeys,
            crate::services::output::OutputService::new(),
            PermissionsService::new(),
            AudioService::new(),
        )
    }

    #[test]
    fn rejects_invalid_or_conflicting_hotkeys() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        let err = ctrl
            .set_hotkeys("Ctrl+Shift".to_string(), "Ctrl+K".to_string())
            .unwrap_err();
        assert!(err.contains("missing non-modifier key"));

        ctrl.set_hotkeys("CmdOrCtrl+P".to_string(), "Ctrl".to_string())
            .expect("dictate should allow modifier-only hotkey");

        let err = ctrl
            .set_hotkeys("CmdOrCtrl+S".to_string(), "cmdorctrl+s".to_string())
            .unwrap_err();
        assert!(err.contains("conflict"));
    }

    #[test]
    fn persists_hotkeys_and_reloads_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let config = ConfigService::load(config_path.clone()).unwrap();
        let ctrl = make_controller(config, None);

        ctrl.set_hotkeys("CmdOrCtrl+Shift+S".to_string(), "Ctrl+D".to_string())
            .unwrap();

        let reloaded = ConfigService::load(config_path).unwrap();
        let cfg = reloaded.get();
        assert_eq!(cfg.open_scribe_hotkey, "CmdOrCtrl+Shift+S");
        assert_eq!(cfg.dictate_hotkey, "Ctrl+D");
    }

    #[test]
    fn set_output_path_persists_and_reloads() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let output_dir = tmp.path().join("out");
        let config = ConfigService::load(config_path.clone()).unwrap();
        let ctrl = make_controller(config, None);

        ctrl.set_output_path(output_dir.to_string_lossy().to_string())
            .unwrap();

        let reloaded = ConfigService::load(config_path).unwrap();
        let cfg = reloaded.get();
        assert_eq!(
            cfg.save_folder,
            std::fs::canonicalize(&output_dir)
                .unwrap()
                .to_string_lossy()
                .to_string()
        );
    }

    #[test]
    fn refuses_hotkey_registration_failure_without_persisting() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let config = ConfigService::load(config_path.clone()).unwrap();
        let ctrl = make_controller(config, Some("CmdOrCtrl+P".to_string()));

        let err = ctrl
            .set_hotkeys("CmdOrCtrl+P".to_string(), "Ctrl+D".to_string())
            .unwrap_err();
        assert!(err.contains("mock register failure"));

        let reloaded = ConfigService::load(config_path).unwrap();
        let cfg = reloaded.get();
        assert_eq!(
            cfg.open_scribe_hotkey,
            crate::types::Config::default().open_scribe_hotkey
        );
    }

    #[test]
    fn set_input_labels_trims_and_persists_values() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        ctrl.set_input_labels("  Podcast Mic  ".to_string(), "  Monitor  ".to_string())
            .expect("set labels");
        let (input, output) = ctrl.get_input_labels();
        assert_eq!(input, "Podcast Mic");
        assert_eq!(output, "Monitor");
    }

    #[test]
    fn set_input_labels_rejects_empty_labels() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        assert!(ctrl
            .set_input_labels("   ".to_string(), "Speaker".to_string())
            .is_err());
    }

    #[test]
    fn preferred_audio_devices_trim_and_persist() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        ctrl.set_preferred_audio_devices(
            Some("  Built-in Mic ".to_string()),
            Some("  BlackHole 2ch ".to_string()),
        )
        .expect("set preferred devices");
        let (preferred_input, preferred_speaker) = ctrl.get_preferred_audio_devices();
        assert_eq!(preferred_input.as_deref(), Some("Built-in Mic"));
        assert_eq!(preferred_speaker.as_deref(), Some("BlackHole 2ch"));
    }

    #[test]
    fn theme_mode_defaults_to_system_and_persists() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let config = ConfigService::load(config_path.clone()).unwrap();
        let ctrl = make_controller(config, None);

        assert_eq!(ctrl.get_theme_mode(), ThemeMode::System);
        ctrl.set_theme_mode("light".to_string())
            .expect("set theme mode");
        assert_eq!(ctrl.get_theme_mode(), ThemeMode::Light);

        let reloaded = ConfigService::load(config_path).unwrap();
        let ctrl2 = make_controller(reloaded, None);
        assert_eq!(ctrl2.get_theme_mode(), ThemeMode::Light);
    }

    #[test]
    fn theme_mode_rejects_unknown_values() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        assert!(ctrl.set_theme_mode("sepia".to_string()).is_err());
    }

    #[test]
    fn onboarding_starts_incomplete_and_completes() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("config.json");
        let config = ConfigService::load(config_path.clone()).unwrap();
        let ctrl = make_controller(config, None);

        assert!(!ctrl.is_onboarding_complete());
        ctrl.complete_onboarding().expect("complete onboarding");
        assert!(ctrl.is_onboarding_complete());

        let reloaded = ConfigService::load(config_path).unwrap();
        let ctrl2 = make_controller(reloaded, None);
        assert!(ctrl2.is_onboarding_complete(), "persisted across reload");
    }

    #[test]
    fn reset_onboarding_reverts_to_incomplete() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        ctrl.complete_onboarding().unwrap();
        assert!(ctrl.is_onboarding_complete());
        ctrl.reset_onboarding().unwrap();
        assert!(!ctrl.is_onboarding_complete());
    }

    #[test]
    fn set_output_path_rejects_empty_and_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let config = ConfigService::load(tmp.path().join("config.json")).unwrap();
        let ctrl = make_controller(config, None);

        assert!(ctrl.set_output_path("".to_string()).is_err());
        assert!(ctrl.set_output_path("   ".to_string()).is_err());
        assert!(ctrl.set_output_path("relative/path".to_string()).is_err());
    }
}
