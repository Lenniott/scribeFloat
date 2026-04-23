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
            let data = std::fs::read_to_string(&path)
                .context("failed to read config file")?;
            serde_json::from_str::<Config>(&data).unwrap_or_default()
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
        self.inner.read().unwrap().clone()
    }

    /// Mutate config via closure then persist atomically.
    pub fn update<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.inner.write().unwrap();
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
