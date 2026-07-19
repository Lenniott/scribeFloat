use crate::services::model::ModelService;
use std::sync::Arc;

pub struct ModelController {
    model: Arc<ModelService>,
}

impl ModelController {
    pub fn new(model: Arc<ModelService>) -> Arc<Self> {
        Arc::new(Self { model })
    }

    pub fn vad_model_status(&self) -> bool {
        self.model.bundled_vad_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn vad_model_status_false_when_missing() {
        let model = ModelService::new(temp_dir("liscribe-model-controller-models"));
        let ctrl = ModelController::new(model);
        assert!(!ctrl.vad_model_status());
    }
}
