use crate::types::{ModelDownloadEvent, Segment};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub const SMALL_MODEL_FILENAME: &str = "ggml-small.bin";

#[derive(Clone, Copy)]
pub struct ModelCatalogItem {
    pub id: &'static str,
    pub label: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
}

const MODEL_CATALOG: [ModelCatalogItem; 4] = [
    ModelCatalogItem {
        id: "tiny",
        label: "Tiny",
        file_name: "ggml-tiny.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    },
    ModelCatalogItem {
        id: "base",
        label: "Base",
        file_name: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    },
    ModelCatalogItem {
        id: "small",
        label: "Small",
        file_name: SMALL_MODEL_FILENAME,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
    ModelCatalogItem {
        id: "medium",
        label: "Medium",
        file_name: "ggml-medium.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
];

pub struct ModelService {
    models_dir: PathBuf,
}

impl ModelService {
    pub fn new(models_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self { models_dir })
    }

    /// Path where the default small model lives on disk.
    pub fn default_model_path(&self) -> PathBuf {
        self.models_dir.join(SMALL_MODEL_FILENAME)
    }

    pub fn model_catalog(&self) -> &'static [ModelCatalogItem] {
        &MODEL_CATALOG
    }

    pub fn model_path_for_id(&self, model_id: &str) -> Option<PathBuf> {
        self.catalog_item(model_id)
            .map(|item| self.models_dir.join(item.file_name))
    }

    pub fn default_model_ready(&self) -> bool {
        self.default_model_path().exists()
    }

    pub fn model_available(&self, path: &Path) -> bool {
        path.exists() && std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
    }

    pub fn model_downloaded(&self, model_id: &str) -> bool {
        self.model_path_for_id(model_id)
            .map(|p| self.model_available(&p))
            .unwrap_or(false)
    }

    /// Download ggml-small.bin into the models directory.
    /// Writes to a .tmp file and renames on success so a failed download
    /// never leaves a corrupt model on disk.
    /// Emits `model://download-progress` events throughout.
    pub async fn download_default(&self, app: &AppHandle) -> Result<()> {
        self.download_model("small", app).await
    }

    pub async fn download_model(&self, model_id: &str, app: &AppHandle) -> Result<()> {
        let item = self
            .catalog_item(model_id)
            .ok_or_else(|| anyhow!("unknown model id: {model_id}"))?;
        let dest = self.models_dir.join(item.file_name);
        let tmp = dest.with_extension("tmp");

        std::fs::create_dir_all(&self.models_dir).context("create models dir")?;

        let client = reqwest::Client::builder()
            .user_agent("liscribe_v8/0.1")
            .timeout(Duration::from_secs(60))
            .build()
            .context("failed to build download client")?;

        let mut response = None;
        let mut last_err = None;
        for attempt in 1..=3 {
            match client.get(item.url).send().await {
                Ok(r) if r.status().is_success() => {
                    response = Some(r);
                    break;
                }
                Ok(r) if r.status().is_server_error() && attempt < 3 => {
                    last_err = Some(anyhow!("server error {} on attempt {attempt}", r.status()));
                    tokio::time::sleep(Duration::from_millis(600 * attempt as u64)).await;
                }
                Ok(r) => {
                    last_err = Some(anyhow!("model download failed with HTTP {}", r.status()));
                    break;
                }
                Err(e) if attempt < 3 => {
                    last_err = Some(anyhow!("download request failed on attempt {attempt}: {e}"));
                    tokio::time::sleep(Duration::from_millis(600 * attempt as u64)).await;
                }
                Err(e) => {
                    last_err = Some(anyhow!("download request failed: {e}"));
                    break;
                }
            }
        }
        let mut response = response.ok_or_else(|| {
            anyhow!(
                "failed to download default model after retries: {}",
                last_err
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "unknown error".to_string())
            )
        })?;

        let total = response.content_length();
        let mut downloaded = 0u64;

        let mut file = tokio::fs::File::create(&tmp)
            .await
            .context("failed to create temp file")?;

        while let Some(chunk) = response.chunk().await.context("stream read error")? {
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;
            app.emit(
                "model://download-progress",
                ModelDownloadEvent {
                    model_id: item.id.to_string(),
                    progress: total.map(|t| downloaded as f32 / t as f32).unwrap_or(0.0),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                },
            )
            .ok();
        }

        // Flush before rename
        tokio::io::AsyncWriteExt::flush(&mut file).await?;
        drop(file);

        tokio::fs::rename(&tmp, &dest)
            .await
            .context("failed to move model into place")?;

        app.emit(
            "model://download-progress",
            ModelDownloadEvent {
                model_id: item.id.to_string(),
                progress: 1.0,
                bytes_downloaded: downloaded,
                total_bytes: total,
            },
        )
        .ok();

        Ok(())
    }

    fn catalog_item(&self, model_id: &str) -> Option<ModelCatalogItem> {
        MODEL_CATALOG.iter().find(|m| m.id == model_id).copied()
    }

    /// Transcribe mono f32 PCM at 16 kHz. Must be called from spawn_blocking.
    pub fn transcribe_pcm(&self, model_path: &Path, pcm: &[f32]) -> Result<Vec<Segment>> {
        let path_str = model_path
            .to_str()
            .ok_or_else(|| anyhow!("model path is not valid UTF-8"))?;

        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
            .map_err(|e| anyhow!("failed to load model at {path_str}: {e:?}"))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow!("failed to create whisper state: {e:?}"))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_single_segment(false);

        state
            .full(params, pcm)
            .map_err(|e| anyhow!("whisper inference failed: {e:?}"))?;

        let n = state
            .full_n_segments()
            .map_err(|e| anyhow!("full_n_segments: {e:?}"))?;

        let mut segments = Vec::with_capacity(n as usize);
        for i in 0..n {
            let text = state
                .full_get_segment_text(i)
                .map_err(|e| anyhow!("segment text {i}: {e:?}"))?;
            let t0 = state
                .full_get_segment_t0(i)
                .map_err(|e| anyhow!("segment t0 {i}: {e:?}"))?;
            let t1 = state
                .full_get_segment_t1(i)
                .map_err(|e| anyhow!("segment t1 {i}: {e:?}"))?;
            let text = text.trim().to_string();
            if !text.is_empty() {
                segments.push(Segment {
                    start_ms: t0 * 10,
                    end_ms: t1 * 10,
                    text,
                });
            }
        }

        Ok(segments)
    }
}
