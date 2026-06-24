use crate::services::voiceprint::{profile_summary, VoiceprintService};
use crate::types::{VoiceprintModelStatus, VoiceprintProfileSummary};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub struct VoiceprintController {
    service: Arc<VoiceprintService>,
}

impl VoiceprintController {
    pub fn new(service: Arc<VoiceprintService>) -> Arc<Self> {
        Arc::new(Self { service })
    }

    pub fn list_profiles(&self) -> Result<Vec<VoiceprintProfileSummary>, String> {
        Ok(self
            .service
            .load_profiles()
            .map_err(|e| e.to_string())?
            .iter()
            .map(profile_summary)
            .collect())
    }

    pub fn list_profile_names(&self) -> Result<Vec<String>, String> {
        Ok(self
            .service
            .load_profiles()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|profile| profile.name)
            .collect())
    }

    pub fn delete_profile(&self, slug: String) -> Result<(), String> {
        let slug = slug.trim();
        if slug.is_empty() {
            return Err("voiceprint profile slug is required".to_string());
        }
        self.service.delete_profile(slug).map_err(|e| e.to_string())
    }

    pub fn rename_profile(&self, slug: String, name: String) -> Result<(), String> {
        let slug = slug.trim();
        let name = name.trim();
        if slug.is_empty() {
            return Err("voiceprint profile slug is required".to_string());
        }
        if name.is_empty() {
            return Err("voiceprint profile name cannot be empty".to_string());
        }
        self.service
            .rename_profile(slug, name)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn model_status(&self) -> VoiceprintModelStatus {
        VoiceprintModelStatus {
            downloaded: self.service.model_downloaded(),
            path: self.service.model_path().to_string_lossy().to_string(),
        }
    }

    pub fn download_model(self: Arc<Self>, app: AppHandle) -> Result<(), String> {
        tauri::async_runtime::spawn(async move {
            if let Err(err) = self.service.download_model(&app).await {
                tracing::warn!(error = %err, "voiceprint model download failed");
                app.emit("voiceprint://model-download-error", err.to_string())
                    .ok();
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::voiceprint::VOICEPRINT_MODEL_FILE;
    use std::path::PathBuf;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn list_profiles_returns_empty_when_store_is_empty() {
        let root = temp_dir("scribefloat-voiceprint-controller");
        let svc = VoiceprintService::new(
            &root.join(VOICEPRINT_MODEL_FILE),
            &root.join("profiles"),
            0.75,
        )
        .unwrap();
        let ctrl = VoiceprintController::new(Arc::new(svc));

        assert!(ctrl.list_profiles().unwrap().is_empty());
    }
}
