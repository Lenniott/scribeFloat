use crate::types::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct ConfigService {
    path: PathBuf,
    inner: RwLock<Config>,
}

impl ConfigService {
    /// Load config from disk, or create with defaults if the file doesn't exist.
    pub fn load(path: PathBuf) -> Result<Arc<Self>> {
        let config = if path.exists() {
            let data = std::fs::read_to_string(&path).context("failed to read config file")?;
            match serde_json::from_str::<Config>(&data) {
                Ok(config) => config,
                Err(err) => {
                    // Don't silently clobber a corrupt-but-possibly-recoverable file on the next
                    // save: move it aside first so the user can salvage their settings.
                    let backup = path.with_extension("json.corrupt");
                    let _ = std::fs::rename(&path, &backup);
                    tracing::warn!(
                        path = %path.display(), backup = %backup.display(), error = %err,
                        "corrupt config; backed up and loaded defaults"
                    );
                    Config::default()
                }
            }
        } else {
            Config::default()
        };
        Ok(Arc::new(Self {
            path,
            inner: RwLock::new(config),
        }))
    }

    /// Cheap clone of current config for use at call site.
    pub fn get(&self) -> Config {
        self.inner.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Mutate config via closure then persist atomically.
    pub fn update<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.inner.write().unwrap_or_else(|p| p.into_inner());
        f(&mut config);
        self.save(&config)
    }

    fn save(&self, config: &Config) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let data = serde_json::to_string_pretty(config)?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config_path() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liscribe-config-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join("config.json")
    }

    #[test]
    fn update_persists_input_labels_across_reload() {
        let path = temp_config_path();
        let service = ConfigService::load(path.clone()).expect("load config");
        service
            .update(|cfg| {
                cfg.input_label = "Desk Mic".to_string();
                cfg.output_label = "Room Speaker".to_string();
            })
            .expect("update labels");

        let reloaded = ConfigService::load(path).expect("reload config");
        let cfg = reloaded.get();
        assert_eq!(cfg.input_label, "Desk Mic");
        assert_eq!(cfg.output_label, "Room Speaker");
    }

    #[test]
    fn update_persists_timestamp_toggle_across_reload() {
        let path = temp_config_path();
        let service = ConfigService::load(path.clone()).expect("load config");
        service
            .update(|cfg| cfg.include_timestamps = false)
            .expect("update timestamp flag");

        let reloaded = ConfigService::load(path).expect("reload config");
        assert!(!reloaded.get().include_timestamps);
    }

    #[test]
    fn loads_defaults_when_config_file_is_missing() {
        let path = temp_config_path();
        // Do not create the file — simulates first-run.
        let service = ConfigService::load(path).expect("load from missing file");
        let cfg = service.get();
        assert!(cfg.include_timestamps);
        assert!(!cfg.onboarding_complete);
        assert!(!cfg.keep_wav);
        assert!(cfg.save_folder.contains("transcripts_scribefloat"));
    }

    #[test]
    fn loads_defaults_for_missing_fields_in_old_config() {
        let path = temp_config_path();
        // Write a minimal config that predates newer fields.
        std::fs::write(&path, r#"{"save_folder": "/tmp/old-liscribe"}"#).expect("write old config");

        let service = ConfigService::load(path).expect("load partial config");
        let cfg = service.get();
        assert_eq!(cfg.save_folder, "/tmp/old-liscribe");
        assert!(cfg.include_timestamps, "should default to true");
        assert!(!cfg.onboarding_complete, "should default to false");
        assert_eq!(cfg.open_scribe_hotkey, "CmdOrCtrl+Shift+L");
        assert_eq!(cfg.input_label, "Mic");
        assert_eq!(cfg.output_label, "Speaker");
        assert_eq!(cfg.theme_mode, crate::types::ThemeMode::System);
    }

    #[test]
    fn falls_back_to_defaults_when_config_is_corrupt() {
        let path = temp_config_path();
        std::fs::write(&path, b"not valid json at all!!!").expect("write corrupt config");

        let service = ConfigService::load(path).expect("load should not fail on corrupt config");
        let cfg = service.get();
        assert!(cfg.include_timestamps);
    }

    #[test]
    fn onboarding_complete_persists_across_reload() {
        let path = temp_config_path();
        let service = ConfigService::load(path.clone()).expect("load config");
        assert!(!service.get().onboarding_complete);

        service
            .update(|cfg| cfg.onboarding_complete = true)
            .expect("mark onboarding complete");

        let reloaded = ConfigService::load(path).expect("reload");
        assert!(reloaded.get().onboarding_complete);
    }
}
