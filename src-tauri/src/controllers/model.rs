use crate::services::config::ConfigService;
use crate::services::model::ModelService;
use crate::types::ModelListItem;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct ModelController {
    model: Arc<ModelService>,
    config: Arc<ConfigService>,
}

impl ModelController {
    pub fn new(model: Arc<ModelService>, config: Arc<ConfigService>) -> Arc<Self> {
        Arc::new(Self { model, config })
    }

    pub fn setup_status(&self) -> bool {
        self.model
            .model_catalog()
            .iter()
            .any(|item| self.model.model_downloaded(item.id))
    }

    pub fn list_models(&self) -> Vec<ModelListItem> {
        let cfg = self.config.get();
        self.model
            .model_catalog()
            .iter()
            .map(|item| {
                let path = self.model.model_path_for_id(item.id);
                let selected = cfg.selected_model_id.as_deref() == Some(item.id)
                    || cfg
                        .scribe_model_path
                        .as_ref()
                        .is_some_and(|configured_path| {
                            path.as_ref()
                                .map(|p| p.to_string_lossy().as_ref() == configured_path)
                                .unwrap_or(false)
                        });

                ModelListItem {
                    id: item.id.to_string(),
                    label: item.label.to_string(),
                    file_name: item.file_name.to_string(),
                    downloaded: self.model.model_downloaded(item.id),
                    selected,
                }
            })
            .collect()
    }

    pub fn download_model(self: Arc<Self>, model_id: String, app: AppHandle) -> Result<(), String> {
        if self.model.model_path_for_id(&model_id).is_none() {
            return Err(format!("unknown model id: {model_id}"));
        }
        tauri::async_runtime::spawn(async move {
            if let Err(e) = self.model.download_model(&model_id, &app).await {
                eprintln!("model download failed: {e}");
                app.emit("model://download-error", e.to_string()).ok();
            }
        });
        Ok(())
    }

    pub fn select_model(&self, model_id: String) -> Result<(), String> {
        let path = self
            .model
            .model_path_for_id(&model_id)
            .ok_or_else(|| format!("unknown model id: {model_id}"))?;
        if !self.model.model_available(&path) {
            return Err(format!("model {model_id} is not downloaded yet"));
        }
        let chosen_path = path.to_string_lossy().to_string();
        self.config
            .update(|cfg| {
                cfg.selected_model_id = Some(model_id.clone());
                cfg.scribe_model_path = Some(chosen_path);
            })
            .map_err(|e| e.to_string())
    }

    /// Deletes the downloaded file for `model_id` and clears config if it pointed at that file.
    pub fn remove_model(&self, model_id: String) -> Result<(), String> {
        let id = model_id.trim();
        if id.is_empty() {
            return Err("model id is required".into());
        }
        let resolved = self
            .model
            .model_path_for_id(id)
            .ok_or_else(|| format!("unknown model id: {id}"))?;
        let normalized = resolved
            .canonicalize()
            .unwrap_or_else(|_| resolved.clone());

        let cfg_snap = self.config.get();
        let matches_config =
            cfg_snap.selected_model_id.as_deref() == Some(id)
                || cfg_snap.scribe_model_path.as_ref().is_some_and(|stored| {
                    let p = Path::new(stored);
                    p == resolved.as_path() || p == normalized.as_path()
                });

        self.model.delete_downloaded_model(id)?;

        if matches_config {
            self.config
                .update(|cfg| {
                    cfg.selected_model_id = None;
                    cfg.scribe_model_path = None;
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::config::ConfigService;
    use std::path::PathBuf;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn select_model_requires_downloaded_file() {
        let models_dir = temp_dir("liscribe-model-controller-models");
        let config_path = temp_dir("liscribe-model-controller-config").join("config.json");
        let model = ModelService::new(models_dir);
        let config = ConfigService::load(config_path).expect("load config");
        let ctrl = ModelController::new(model, config);

        let err = ctrl
            .select_model("tiny".to_string())
            .expect_err("should reject missing model");
        assert!(err.contains("not downloaded"));
    }

    #[test]
    fn select_model_persists_selected_id_and_path() {
        let models_dir = temp_dir("liscribe-model-controller-models");
        let config_path = temp_dir("liscribe-model-controller-config").join("config.json");
        let tiny_path = models_dir.join("ggml-tiny.bin");
        std::fs::write(&tiny_path, [1, 2, 3]).expect("write model file");

        let model = ModelService::new(models_dir.clone());
        let config = ConfigService::load(config_path).expect("load config");
        let ctrl = ModelController::new(Arc::clone(&model), Arc::clone(&config));

        ctrl.select_model("tiny".to_string())
            .expect("select downloaded model");

        let cfg = config.get();
        assert_eq!(cfg.selected_model_id.as_deref(), Some("tiny"));
        assert_eq!(
            cfg.scribe_model_path.as_deref(),
            Some(tiny_path.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn remove_requires_download() {
        let models_dir = temp_dir("liscribe-model-remove-missing-models");
        let config_path = temp_dir("liscribe-model-remove-missing-config").join("config.json");
        let model = ModelService::new(models_dir);
        let config = ConfigService::load(config_path).expect("load config");
        let ctrl = ModelController::new(model, config);

        let err = ctrl
            .remove_model("tiny".to_string())
            .expect_err("should reject missing file");
        assert!(err.contains("not downloaded"));
    }

    #[test]
    fn remove_deletes_and_clears_when_selected() {
        let models_dir = temp_dir("liscribe-model-remove-models");
        let config_path = temp_dir("liscribe-model-remove-config").join("config.json");
        let tiny_path = models_dir.join("ggml-tiny.bin");
        std::fs::write(&tiny_path, [7, 7, 7]).expect("write model file");

        let model = ModelService::new(models_dir.clone());
        let config = ConfigService::load(config_path).expect("load config");
        let ctrl = ModelController::new(Arc::clone(&model), Arc::clone(&config));

        ctrl.select_model("tiny".to_string()).expect("select");
        ctrl.remove_model("tiny".to_string()).expect("remove");

        assert!(!tiny_path.exists(), "binary should be deleted");
        let cfg = config.get();
        assert!(cfg.selected_model_id.is_none());
        assert!(cfg.scribe_model_path.is_none());
    }
}
