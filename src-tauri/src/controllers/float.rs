use crate::services::config::ConfigService;
use crate::services::inference::InferenceService;
use crate::types::{FloatConfig, FloatModelInfo, FloatProvider};
use std::sync::Arc;

pub struct FloatController {
    inference: Arc<InferenceService>,
    config: Arc<ConfigService>,
}

impl FloatController {
    pub fn new(inference: Arc<InferenceService>, config: Arc<ConfigService>) -> Arc<Self> {
        Arc::new(Self { inference, config })
    }

    /// Return the current Float configuration (safe for the frontend — API key is not included).
    pub fn get_config(&self) -> FloatConfig {
        let cfg = self.config.get();
        FloatConfig {
            provider: cfg.float_provider,
            endpoint_url: cfg.float_endpoint_url.clone(),
            has_api_key: cfg.float_api_key.is_some(),
            model: cfg.float_model.clone(),
            ready: cfg.float_model.is_some(),
        }
    }

    /// Persist Float provider settings. Switches `endpoint_url` to the provider's default
    /// when the caller supplies an empty string, so the frontend doesn't have to know defaults.
    pub fn set_config(
        &self,
        provider: String,
        endpoint_url: String,
        api_key: Option<String>,
        model: Option<String>,
    ) -> Result<(), String> {
        let provider = FloatProvider::parse(&provider)?;
        let endpoint_url = if endpoint_url.trim().is_empty() {
            provider.default_endpoint().to_string()
        } else {
            endpoint_url.trim_end_matches('/').to_string()
        };

        self.config
            .update(|cfg| {
                cfg.float_provider = provider;
                cfg.float_endpoint_url = endpoint_url;
                // Only overwrite the stored key when the caller explicitly provides one.
                // `None` means "leave the existing key unchanged" so the frontend can update
                // endpoint/model without clearing a previously saved key.
                if api_key.is_some() {
                    cfg.float_api_key = api_key;
                }
                cfg.float_model = model;
            })
            .map_err(|e| e.to_string())
    }

    /// Clear the stored API key (separate from set_config so the frontend can offer
    /// a dedicated "remove key" action without resetting other settings).
    pub fn clear_api_key(&self) -> Result<(), String> {
        self.config
            .update(|cfg| {
                cfg.float_api_key = None;
            })
            .map_err(|e| e.to_string())
    }

    /// Fetch available models from the configured provider.
    pub async fn list_models(&self) -> Result<Vec<FloatModelInfo>, String> {
        let cfg = self.config.get();
        self.inference
            .list_models(
                cfg.float_provider,
                &cfg.float_endpoint_url,
                cfg.float_api_key.as_deref(),
            )
            .await
            .map_err(|e| e.to_string())
    }

    /// Probe the endpoint by listing models. Returns Ok(()) if at least one model is
    /// reachable, Err with a user-facing message otherwise.
    pub async fn test_connection(&self) -> Result<(), String> {
        let models = self.list_models().await?;
        if models.is_empty() {
            return Err(
                "Connected but no models found — install a model in Ollama or check your API key."
                    .to_string(),
            );
        }
        Ok(())
    }
}
