use crate::services::config::ConfigService;
use crate::services::model::ModelService;
use std::sync::Arc;

pub struct ModelController {
    model: Arc<ModelService>,
    config: Arc<ConfigService>,
}

impl ModelController {
    pub fn new(model: Arc<ModelService>, config: Arc<ConfigService>) -> Arc<Self> {
        Arc::new(Self { model, config })
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

    pub fn vad_model_status(&self) -> bool {
        self.model.vad_model_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::config::ConfigService;
    use crate::services::model::{DEFAULT_MODEL_ID, SMALL_MODEL_FILENAME};
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
            .select_model(DEFAULT_MODEL_ID.to_string())
            .expect_err("should reject missing model");
        assert!(err.contains("not downloaded"));
    }

    #[test]
    fn select_model_rejects_removed_catalog_ids() {
        let models_dir = temp_dir("liscribe-model-controller-models");
        let config_path = temp_dir("liscribe-model-controller-config").join("config.json");
        let model = ModelService::new(models_dir);
        let config = ConfigService::load(config_path).expect("load config");
        let ctrl = ModelController::new(model, config);

        let err = ctrl
            .select_model("base-en-q5".to_string())
            .expect_err("removed catalog entry should be unknown");
        assert!(err.contains("unknown model id"));
    }

    #[test]
    fn select_model_persists_selected_id_and_path() {
        let models_dir = temp_dir("liscribe-model-controller-models");
        let config_path = temp_dir("liscribe-model-controller-config").join("config.json");
        let small_path = models_dir.join(SMALL_MODEL_FILENAME);
        std::fs::write(&small_path, [1, 2, 3]).expect("write model file");

        let model = ModelService::new(models_dir.clone());
        let config = ConfigService::load(config_path).expect("load config");
        let ctrl = ModelController::new(Arc::clone(&model), Arc::clone(&config));

        ctrl.select_model(DEFAULT_MODEL_ID.to_string())
            .expect("select downloaded model");

        let cfg = config.get();
        assert_eq!(cfg.selected_model_id.as_deref(), Some(DEFAULT_MODEL_ID));
        assert_eq!(
            cfg.scribe_model_path.as_deref(),
            Some(small_path.to_string_lossy().as_ref())
        );
    }
}
