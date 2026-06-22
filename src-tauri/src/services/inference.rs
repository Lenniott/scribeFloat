use crate::types::{FloatModelInfo, FloatProvider};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// Stateless HTTP client for inference provider interactions.
///
/// Holds one `reqwest::Client` for connection-pool reuse. Caller (FloatController)
/// reads Config and passes the provider/endpoint/key explicitly so this service
/// has no config dependency and stays fully testable.
pub struct InferenceService {
    client: reqwest::Client,
}

impl InferenceService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("scribefloat/float-engine")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("failed to build inference HTTP client");
        Self { client }
    }

    /// List models available from the configured provider.
    pub async fn list_models(
        &self,
        provider: FloatProvider,
        endpoint: &str,
        api_key: Option<&str>,
    ) -> Result<Vec<FloatModelInfo>> {
        match provider {
            FloatProvider::Ollama => self.list_models_ollama(endpoint).await,
            FloatProvider::OpenAi => {
                self.list_models_openai_compat(endpoint, api_key, true).await
            }
            FloatProvider::Anthropic => self.list_models_anthropic(endpoint, api_key).await,
            FloatProvider::Custom => {
                self.list_models_openai_compat(endpoint, api_key, false).await
            }
        }
    }

    // ── Provider implementations ──────────────────────────────────────────────

    async fn list_models_ollama(&self, endpoint: &str) -> Result<Vec<FloatModelInfo>> {
        // Ollama native API: GET /api/tags
        // Response: { "models": [{ "name": "llama3.2:3b", "size": ..., ... }] }
        #[derive(Deserialize)]
        struct OllamaModel {
            name: String,
        }
        #[derive(Deserialize)]
        struct OllamaTagsResponse {
            models: Vec<OllamaModel>,
        }

        let url = format!("{endpoint}/api/tags");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("failed to reach Ollama — is it running?")?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "Ollama returned HTTP {} for GET /api/tags",
                resp.status()
            ));
        }

        let body: OllamaTagsResponse = resp
            .json()
            .await
            .context("failed to parse Ollama /api/tags response")?;

        Ok(body
            .models
            .into_iter()
            .map(|m| FloatModelInfo {
                label: m.name.clone(),
                id: m.name,
            })
            .collect())
    }

    async fn list_models_openai_compat(
        &self,
        endpoint: &str,
        api_key: Option<&str>,
        filter_chat_models: bool,
    ) -> Result<Vec<FloatModelInfo>> {
        // OpenAI-compatible: GET /v1/models
        // Response: { "data": [{ "id": "gpt-4o-mini", ... }] }
        #[derive(Deserialize)]
        struct OpenAiModel {
            id: String,
        }
        #[derive(Deserialize)]
        struct OpenAiModelsResponse {
            data: Vec<OpenAiModel>,
        }

        let url = format!("{endpoint}/v1/models");
        let mut req = self.client.get(&url);

        if let Some(key) = api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .await
            .context("failed to reach inference endpoint")?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "provider returned HTTP {} for GET /v1/models",
                resp.status()
            ));
        }

        let body: OpenAiModelsResponse = resp
            .json()
            .await
            .context("failed to parse /v1/models response")?;

        let models: Vec<FloatModelInfo> = body
            .data
            .into_iter()
            .filter(|m| {
                if filter_chat_models {
                    // Keep only chat/completions-capable models; skip embeddings, TTS, etc.
                    !m.id.contains("embed")
                        && !m.id.contains("tts")
                        && !m.id.contains("dall-e")
                        && !m.id.contains("whisper")
                } else {
                    true
                }
            })
            .map(|m| FloatModelInfo {
                label: m.id.clone(),
                id: m.id,
            })
            .collect();

        Ok(models)
    }

    async fn list_models_anthropic(
        &self,
        endpoint: &str,
        api_key: Option<&str>,
    ) -> Result<Vec<FloatModelInfo>> {
        // Anthropic: GET /v1/models
        // Headers: x-api-key, anthropic-version
        // Response: { "data": [{ "id": "claude-...", "display_name": "Claude ..." }] }
        #[derive(Deserialize)]
        struct AnthropicModel {
            id: String,
            display_name: String,
        }
        #[derive(Deserialize)]
        struct AnthropicModelsResponse {
            data: Vec<AnthropicModel>,
        }

        let key = api_key.ok_or_else(|| anyhow!("Anthropic requires an API key"))?;
        let url = format!("{endpoint}/v1/models");

        let resp = self
            .client
            .get(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .context("failed to reach Anthropic API")?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "Anthropic returned HTTP {} for GET /v1/models",
                resp.status()
            ));
        }

        let body: AnthropicModelsResponse = resp
            .json()
            .await
            .context("failed to parse Anthropic /v1/models response")?;

        Ok(body
            .data
            .into_iter()
            .map(|m| FloatModelInfo {
                label: m.display_name,
                id: m.id,
            })
            .collect())
    }
}
