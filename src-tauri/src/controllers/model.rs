use crate::services::config::ConfigService;
use crate::services::model::ModelService;
use crate::types::ModelListItem;
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
                    || cfg.scribe_model_path.as_ref().is_some_and(|configured_path| {
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

    pub fn download_model(self: Arc<Self>, model_id: String, app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            if let Err(e) = self.model.download_model(&model_id, &app).await {
                eprintln!("model download failed: {e}");
                app.emit("model://download-error", e.to_string()).ok();
            }
        });
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
}
