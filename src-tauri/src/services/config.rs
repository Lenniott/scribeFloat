use crate::types::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct ConfigService {
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
            inner: RwLock::new(config),
        }))
    }

    /// Cheap clone of current config for use at call site.
    pub fn get(&self) -> Config {
        self.inner.read().unwrap().clone()
    }

}
