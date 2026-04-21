use crate::types::{ModelDownloadEvent, Segment};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const SMALL_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";
pub const SMALL_MODEL_FILENAME: &str = "ggml-small.bin";

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

    pub fn default_model_ready(&self) -> bool {
        self.default_model_path().exists()
    }

    pub fn model_available(&self, path: &Path) -> bool {
        path.exists()
    }

    /// Download ggml-small.bin into the models directory.
    /// Writes to a .tmp file and renames on success so a failed download
    /// never leaves a corrupt model on disk.
    /// Emits `model://download-progress` events throughout.
    pub async fn download_default(&self, app: &AppHandle) -> Result<()> {
        let dest = self.default_model_path();
        let tmp = dest.with_extension("tmp");

        std::fs::create_dir_all(&self.models_dir).context("create models dir")?;

        let client = reqwest::Client::new();
        let response = client
            .get(SMALL_MODEL_URL)
            .send()
            .await
            .context("download request failed")?
            .error_for_status()
            .context("server returned error")?;

        let total = response.content_length();
        let mut downloaded = 0u64;

        let mut file = tokio::fs::File::create(&tmp)
            .await
            .context("failed to create temp file")?;

        let mut response = response;
        while let Some(chunk) = response.chunk().await.context("stream read error")? {
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;
            app.emit(
                "model://download-progress",
                ModelDownloadEvent {
                    model_name: "small".to_string(),
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
                model_name: "small".to_string(),
                progress: 1.0,
                bytes_downloaded: downloaded,
                total_bytes: total,
            },
        )
        .ok();

        Ok(())
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
