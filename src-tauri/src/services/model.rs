use crate::types::{ModelDownloadEvent, Segment};
use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use whisper_rs::{
    FullParams, SamplingStrategy, SegmentCallbackData, WhisperContext, WhisperContextParameters,
    WhisperError, WhisperVadParams,
};

/// Cap inference threads at the number of physical cores. Hyperthreading does not help
/// matmul-heavy workloads and creates lock contention. The 8-thread upper bound matches
/// ggml's own scaling curve — adding threads beyond that on speech-length audio is a wash.
const MAX_INFERENCE_THREADS: usize = 8;

/// Absolute minimum PCM length passed to Whisper (100 ms at 16 kHz).
pub const MIN_PCM_SAMPLES_16K: usize = 1_600;

/// Silero VAD on shorter clips often strips all speech; the encoder then fails with
/// `GenericError(-6)` even though the model and PCM shape are valid.
const VAD_MIN_PCM_SAMPLES: usize = 32_000;

pub const SMALL_MODEL_FILENAME: &str = "ggml-small.en-q5_1.bin";

pub const VAD_MODEL_FILENAME: &str = "ggml-silero-v6.2.0.bin";
const VAD_MODEL_URL: &str =
    "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin";
const VAD_MODEL_SHA256: Option<&str> =
    Some("2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987");

#[derive(Clone, Copy)]
pub struct ModelCatalogItem {
    pub id: &'static str,
    pub label: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub size_mb: u32,
    /// LibriSpeech clean WER % from Open ASR Leaderboard (lower is better).
    pub wer: f32,
    /// Real-time factor from Open ASR Leaderboard on GPU (higher is faster). None = not benchmarked.
    pub rtfx: Option<u32>,
    /// Verified lowercase-hex SHA-256 of the model file. When `Some`, the download is
    /// rejected unless the bytes hash to this value (see `verify_sha256`). When `None`,
    /// the download is accepted unverified.
    pub sha256: Option<&'static str>,
}

const MODEL_CATALOG: [ModelCatalogItem; 5] = [
    ModelCatalogItem {
        id: "tiny-en-q5",
        label: "Tiny",
        file_name: "ggml-tiny.en-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin",
        size_mb: 31,
        wer: 5.66,
        rtfx: Some(348),
        sha256: Some("c77c5766f1cef09b6b7d47f21b546cbddd4157886b3b5d6d4f709e91e66c7c2b"),
    },
    ModelCatalogItem {
        id: "base-en-q5",
        label: "Base",
        file_name: "ggml-base.en-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q5_1.bin",
        size_mb: 57,
        wer: 4.25,
        rtfx: Some(321),
        sha256: Some("4baf70dd0d7c4247ba2b81fafd9c01005ac77c2f9ef064e00dcf195d0e2fdd2f"),
    },
    ModelCatalogItem {
        id: "small-en-q5",
        label: "Small",
        file_name: SMALL_MODEL_FILENAME,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin",
        size_mb: 181,
        wer: 3.05,
        rtfx: Some(269),
        sha256: Some("bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30"),
    },
    ModelCatalogItem {
        id: "medium-en-q5",
        label: "Medium",
        file_name: "ggml-medium.en-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en-q5_0.bin",
        size_mb: 514,
        wer: 3.02,
        rtfx: None,
        sha256: Some("76733e26ad8fe1c7a5bf7531a9d41917b2adc0f20f2e4f5531688a8c6cd88eb0"),
    },
    ModelCatalogItem {
        id: "large-v3-turbo-q5",
        label: "Large Turbo",
        file_name: "ggml-large-v3-turbo-q5_0.bin",
        url:
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
        size_mb: 547,
        wer: 2.10,
        rtfx: Some(200),
        sha256: Some("394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2"),
    },
];

/// Compute the lowercase-hex SHA-256 of a file on disk, streaming it so a multi-hundred-MB
/// model never lands in memory all at once.
fn file_sha256_hex(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

/// Verify a freshly downloaded file against an expected SHA-256. Runs the (CPU-bound) hash on a
/// blocking thread so it never stalls the async runtime. Returns an error on any mismatch.
async fn verify_sha256(path: &Path, expected_hex: &str) -> Result<()> {
    let path = path.to_path_buf();
    let expected = expected_hex.to_ascii_lowercase();
    let actual = tokio::task::spawn_blocking(move || file_sha256_hex(&path))
        .await
        .context("checksum task panicked")?
        .context("failed to hash downloaded file")?;
    if actual != expected {
        return Err(anyhow!(
            "checksum mismatch: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

/// Distinguishes GPU encode failures (retryable on CPU) from other inference errors.
enum InferError {
    Encode(anyhow::Error),
    Other(anyhow::Error),
}

impl From<InferError> for anyhow::Error {
    fn from(e: InferError) -> Self {
        match e {
            InferError::Encode(e) | InferError::Other(e) => e,
        }
    }
}

pub struct ModelService {
    models_dir: PathBuf,
    /// Append-only log of encode failures for post-mortem diagnosis (`{app_data}/transcription-failures.jsonl`).
    failure_log_path: PathBuf,
    /// Loaded Whisper contexts keyed by canonical model path. A `WhisperContext` owns the
    /// model weights (~30 MB tiny → ~550 MB large turbo) and is safe to share across calls —
    /// only the per-inference `WhisperState` is created fresh on each transcribe. Caching
    /// here eliminates the cold-load tax (~300 ms tiny → ~2 s large) that the previous
    /// implementation paid on every `transcribe_pcm_with_progress` call.
    loaded_contexts: Mutex<HashMap<PathBuf, Arc<WhisperContext>>>,
    /// Per-path mutexes so concurrent callers (e.g. record-start preload + stop transcribe)
    /// serialize on the same file instead of each paying a full WhisperContext load.
    loading_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    /// Paths where the GPU encoder previously failed (e.g. Metal encode error on M1).
    /// `get_or_load_context` loads these with `use_gpu = false` from this point forward.
    cpu_fallback_paths: Mutex<HashSet<PathBuf>>,
    /// Serializes all in-flight `whisper_full` calls. `WhisperContext` is shared across
    /// Scribe, Dictate, and Transcribe; concurrent encode passes corrupt Metal/ggml state.
    inference_gate: Mutex<()>,
}

impl ModelService {
    pub fn new(models_dir: PathBuf) -> Arc<Self> {
        let failure_log_path = transcription_failure_log_path(&models_dir);
        Arc::new(Self {
            models_dir,
            failure_log_path,
            loaded_contexts: Mutex::new(HashMap::new()),
            loading_locks: Mutex::new(HashMap::new()),
            cpu_fallback_paths: Mutex::new(HashSet::new()),
            inference_gate: Mutex::new(()),
        })
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

    pub fn model_available(&self, path: &Path) -> bool {
        path.exists()
            && std::fs::metadata(path)
                .map(|m| m.len() > 0)
                .unwrap_or(false)
    }

    pub fn model_downloaded(&self, model_id: &str) -> bool {
        self.model_path_for_id(model_id)
            .map(|p| self.model_available(&p))
            .unwrap_or(false)
    }

    pub fn vad_model_path(&self) -> PathBuf {
        self.models_dir.join(VAD_MODEL_FILENAME)
    }

    pub fn vad_model_available(&self) -> bool {
        self.model_available(&self.vad_model_path())
    }

    /// Whether the on-disk VAD file matches the catalog SHA-256.
    pub fn vad_model_integrity_ok(&self) -> bool {
        let path = self.vad_model_path();
        if !self.vad_model_available() {
            return false;
        }
        let Some(expected) = VAD_MODEL_SHA256 else {
            return true;
        };
        file_sha256_hex(&path)
            .map(|actual| actual == expected.to_ascii_lowercase())
            .unwrap_or(false)
    }

    /// True when VAD is missing or fails the integrity check (stale manual download).
    pub fn vad_model_needs_redownload(&self) -> bool {
        !self.vad_model_available() || !self.vad_model_integrity_ok()
    }

    /// VAD model path when the file is present, passes integrity, and PCM is long enough
    /// for Silero trimming without starving the encoder.
    pub fn vad_path_for_pcm(&self, pcm_samples: usize) -> Option<PathBuf> {
        if pcm_samples < VAD_MIN_PCM_SAMPLES {
            return None;
        }
        self.vad_path_for_inference()
    }

    /// VAD model path when the file is present and passes the catalog SHA-256 check.
    pub fn vad_path_for_inference(&self) -> Option<PathBuf> {
        (self.vad_model_available() && self.vad_model_integrity_ok()).then(|| self.vad_model_path())
    }

    /// Whether a catalog Whisper model on disk matches its published SHA-256.
    /// Custom/non-catalog paths skip verification and only require a non-empty file.
    pub fn whisper_model_integrity_ok(&self, model_path: &Path) -> bool {
        if !self.model_available(model_path) {
            return false;
        }
        let Some(expected) = self.catalog_sha256_for_path(model_path) else {
            return true;
        };
        file_sha256_hex(model_path)
            .map(|actual| actual == expected.to_ascii_lowercase())
            .unwrap_or(false)
    }

    pub fn whisper_model_bytes(&self, model_path: &Path) -> u64 {
        std::fs::metadata(model_path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    fn catalog_sha256_for_path(&self, model_path: &Path) -> Option<&'static str> {
        let name = model_path.file_name()?.to_str()?;
        MODEL_CATALOG
            .iter()
            .find(|item| item.file_name == name)
            .and_then(|item| item.sha256)
    }

    pub async fn download_vad_model(&self, app: &AppHandle) -> Result<()> {
        let dest = self.vad_model_path();
        let tmp = dest.with_extension("tmp");

        std::fs::create_dir_all(&self.models_dir).context("create models dir")?;

        let client = reqwest::Client::builder()
            .user_agent("scribefloat/0.1")
            .connect_timeout(Duration::from_secs(15))
            .build()
            .context("failed to build download client")?;

        let mut response = None;
        let mut last_err = None;
        for attempt in 1..=3 {
            match client.get(VAD_MODEL_URL).send().await {
                Ok(r) if r.status().is_success() => {
                    response = Some(r);
                    break;
                }
                Ok(r) if r.status().is_server_error() && attempt < 3 => {
                    last_err = Some(anyhow!("server error {} on attempt {attempt}", r.status()));
                    tokio::time::sleep(Duration::from_millis(600 * attempt as u64)).await;
                }
                Ok(r) => {
                    last_err = Some(anyhow!("vad download failed with HTTP {}", r.status()));
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
                "failed to download VAD model after retries: {}",
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
                    model_id: "vad".to_string(),
                    progress: total.map(|t| downloaded as f32 / t as f32).unwrap_or(0.0),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                },
            )
            .ok();
        }

        if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut file).await {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(anyhow::Error::from(e));
        }
        drop(file);

        if let Some(expected) = VAD_MODEL_SHA256 {
            if let Err(e) = verify_sha256(&tmp, expected).await {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(e).context("VAD model failed integrity check");
            }
        }

        if let Err(e) = tokio::fs::rename(&tmp, &dest)
            .await
            .context("failed to move VAD model into place")
        {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }

        app.emit(
            "model://download-progress",
            ModelDownloadEvent {
                model_id: "vad".to_string(),
                progress: 1.0,
                bytes_downloaded: downloaded,
                total_bytes: total,
            },
        )
        .ok();

        Ok(())
    }

    /// Removes the downloaded file for `model_id`. Only paths under [`Self::models_dir`]
    /// for known catalog entries are touched.
    pub fn delete_vad_model(&self) -> Result<(), String> {
        let path = self.vad_model_path();
        if !self.vad_model_available() {
            return Err("VAD model is not downloaded".into());
        }
        std::fs::remove_file(&path).map_err(|e| format!("failed to remove VAD model: {e}"))?;
        let tmp = path.with_extension("tmp");
        if tmp.is_file() {
            let _ = std::fs::remove_file(tmp);
        }
        Ok(())
    }

    pub fn delete_downloaded_model(&self, model_id: &str) -> Result<(), String> {
        let path = self
            .model_path_for_id(model_id)
            .ok_or_else(|| format!("unknown model id: {model_id}"))?;
        if !self.model_downloaded(model_id) {
            return Err(format!("model {model_id} is not downloaded"));
        }
        std::fs::remove_file(&path).map_err(|e| format!("failed to remove model file: {e}"))?;
        let tmp = path.with_extension("tmp");
        if tmp.is_file() {
            let _ = std::fs::remove_file(tmp);
        }
        Ok(())
    }

    pub async fn download_model(&self, model_id: &str, app: &AppHandle) -> Result<()> {
        let item = self
            .catalog_item(model_id)
            .ok_or_else(|| anyhow!("unknown model id: {model_id}"))?;
        let dest = self.models_dir.join(item.file_name);
        let tmp = dest.with_extension("tmp");

        std::fs::create_dir_all(&self.models_dir).context("create models dir")?;

        let client = reqwest::Client::builder()
            .user_agent("scribefloat/0.1")
            .connect_timeout(Duration::from_secs(15))
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
        if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut file).await {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(anyhow::Error::from(e));
        }
        drop(file);

        if let Some(expected) = item.sha256 {
            if let Err(e) = verify_sha256(&tmp, expected).await {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(e).context("model failed integrity check");
            }
        }

        if let Err(e) = tokio::fs::rename(&tmp, &dest)
            .await
            .context("failed to move model into place")
        {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }

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

    /// Drop all cached WhisperContexts before app exit so Metal GPU resources are
    /// freed while the Rust runtime is still live — prevents the ggml-metal assertion
    /// `[rsets->data count] == 0` that fires when contexts are dropped during
    /// NSApplication teardown after Metal state has already been partially cleaned up.
    pub fn release_contexts(&self) {
        if let Ok(mut guard) = self.loaded_contexts.lock() {
            guard.clear();
        }
    }

    /// Read the model file into the OS page cache without creating a `WhisperContext`.
    /// Safe to run while recording — unlike `get_or_load_context`, this does not touch Metal.
    pub fn warm_model_file_on_disk(model_path: &Path) {
        let _ = std::fs::read(model_path);
    }

    /// Load a Whisper context for `model_path`, or return the cached one. Loads block on
    /// disk I/O and model parsing, so callers should invoke this from `spawn_blocking` (or
    /// off the async runtime). Subsequent calls for the same path are O(hash lookup).
    pub fn get_or_load_context(&self, model_path: &Path) -> Result<Arc<WhisperContext>> {
        if let Some(ctx) = self.cached_context(model_path) {
            return Ok(ctx);
        }

        let path_key = model_path.to_path_buf();
        let load_lock = self.load_lock_for(&path_key);
        let _in_flight = load_lock.lock().unwrap_or_else(|p| p.into_inner());

        if let Some(ctx) = self.cached_context(model_path) {
            return Ok(ctx);
        }

        let path_str = model_path
            .to_str()
            .ok_or_else(|| anyhow!("model path is not valid UTF-8"))?;

        let use_gpu = !self
            .cpu_fallback_paths
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&path_key);
        let mut ctx_params = WhisperContextParameters::default();
        if !use_gpu {
            tracing::info!(
                model = model_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                "loading model on CPU (GPU encode previously failed)"
            );
            ctx_params.use_gpu(false);
        }

        let load_started = Instant::now();
        let ctx = WhisperContext::new_with_params(path_str, ctx_params)
            .map_err(|e| anyhow!("failed to load model at {path_str}: {e:?}"))?;
        let ctx = Arc::new(ctx);
        tracing::debug!(
            model = model_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path_str.to_string()),
            elapsed_ms = load_started.elapsed().as_millis(),
            "model loaded"
        );

        let mut guard = self.lock_contexts();
        Ok(Arc::clone(
            guard.entry(path_key).or_insert_with(|| Arc::clone(&ctx)),
        ))
    }

    fn cached_context(&self, model_path: &Path) -> Option<Arc<WhisperContext>> {
        let guard = self.lock_contexts();
        guard.get(model_path).map(Arc::clone)
    }

    fn load_lock_for(&self, path_key: &Path) -> Arc<Mutex<()>> {
        let mut locks = self.loading_locks.lock().unwrap_or_else(|p| p.into_inner());
        locks
            .entry(path_key.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn lock_contexts(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, Arc<WhisperContext>>> {
        self.loaded_contexts
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    /// Mark a model path for CPU-only inference after a GPU encode failure.
    /// Evicts any cached GPU context so it is reloaded with `use_gpu = false`.
    fn mark_cpu_fallback(&self, path: &Path) {
        let path = path.to_path_buf();
        self.evict_context(&path);
        self.cpu_fallback_paths
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(path);
    }

    /// Drop a cached `WhisperContext` so the next inference loads a fresh one.
    /// Reusing a context after Metal encode failures can poison later passes.
    fn evict_context(&self, model_path: &Path) {
        self.lock_contexts().remove(model_path);
    }

    /// Clear the sticky CPU-only flag for `model_path` so the next transcription
    /// retries GPU. Evicts any cached context so `get_or_load_context` does not
    /// return a CPU-only context left over from a prior failure.
    fn reset_gpu_preference(&self, model_path: &Path) {
        self.cpu_fallback_paths
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(model_path);
        self.evict_context(model_path);
    }

    /// Transcribe mono f32 PCM at 16 kHz and report Whisper's own progress.
    /// Must be called from spawn_blocking.
    /// Pass `vad_model_path` to enable Silero VAD — silence mid-recording is skipped,
    /// preventing hallucinations during pauses.
    ///
    /// On GPU encode failure (e.g. Metal `GenericError(-6)` on M1) the context is evicted and
    /// the inference is retried automatically on CPU. If encode still fails with Silero VAD
    /// enabled, a final retry runs without VAD. GPU is retried again on the next transcription.
    ///
    /// Long audio uses whisper.cpp's internal seek/windowing — do not pre-chunk PCM here.
    ///
    /// `source` identifies the caller workflow (e.g. `"scribe/mic"`) for failure diagnostics.
    pub fn transcribe_pcm_with_progress<F>(
        &self,
        model_path: &Path,
        pcm: &[f32],
        vad_model_path: Option<&Path>,
        abort: Option<Arc<AtomicBool>>,
        source: &str,
        on_progress: F,
    ) -> Result<Vec<Segment>>
    where
        F: FnMut(f32) + Send + 'static,
    {
        let _inference = self
            .inference_gate
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        self.reset_gpu_preference(model_path);

        if let Some(expected) = self.catalog_sha256_for_path(model_path) {
            if !self.whisper_model_integrity_ok(model_path) {
                let actual = file_sha256_hex(model_path).unwrap_or_default();
                return Err(anyhow!(
                    "Whisper model failed SHA-256 integrity check (expected {expected}, got {actual}). Re-download from Settings."
                ));
            }
        } else if !self.model_available(model_path) {
            return Err(anyhow!(
                "Whisper model not found at {}",
                model_path.display()
            ));
        }

        if pcm.is_empty() {
            return Err(anyhow!("cannot transcribe empty PCM buffer"));
        }
        if pcm.len() < MIN_PCM_SAMPLES_16K {
            return Err(anyhow!(
                "recording too short to transcribe ({} samples, need at least {})",
                pcm.len(),
                MIN_PCM_SAMPLES_16K
            ));
        }
        let pcm_diag = pcm_diagnostics(pcm);
        if pcm_diag.nan_count > 0 || pcm_diag.inf_count > 0 {
            return Err(anyhow!(
                "PCM contains invalid samples (nan={}, inf={})",
                pcm_diag.nan_count,
                pcm_diag.inf_count
            ));
        }

        let vad_model_path = vad_model_path.filter(|_| pcm.len() >= VAD_MIN_PCM_SAMPLES);
        if vad_model_path.is_none() && pcm.len() < VAD_MIN_PCM_SAMPLES {
            tracing::debug!(
                source,
                pcm_samples = pcm.len(),
                "VAD disabled for short clip"
            );
        }

        let on_progress = Arc::new(Mutex::new(on_progress));
        let vad_requested = vad_model_path.is_some();
        let mut gpu_retried = false;

        let mut result = self.run_inference(
            model_path,
            pcm,
            vad_model_path,
            abort.clone(),
            Arc::clone(&on_progress),
        );

        if abort.as_ref().is_some_and(|a| a.load(Ordering::SeqCst)) {
            return Ok(Vec::new());
        }

        if matches!(&result, Err(InferError::Encode(_))) && self.uses_gpu_for(model_path) {
            if let Err(InferError::Encode(ref e)) = result {
                self.record_encode_failure(
                    source,
                    model_path,
                    pcm,
                    vad_requested,
                    true,
                    "gpu",
                    false,
                    e,
                );
                tracing::warn!(
                    source,
                    model = model_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    error = %format!("{e:#}"),
                    pcm = %format_pcm_summary(pcm),
                    "whisper encode failed on GPU, retrying on CPU"
                );
            }
            self.mark_cpu_fallback(model_path);
            gpu_retried = true;
            result = self.run_inference(
                model_path,
                pcm,
                vad_model_path,
                abort.clone(),
                Arc::clone(&on_progress),
            );
        }

        if abort.as_ref().is_some_and(|a| a.load(Ordering::SeqCst)) {
            return Ok(Vec::new());
        }

        if matches!(&result, Err(InferError::Encode(_))) && vad_requested {
            if let Err(InferError::Encode(ref e)) = result {
                self.record_encode_failure(
                    source,
                    model_path,
                    pcm,
                    true,
                    !self.uses_gpu_for(model_path),
                    if gpu_retried { "cpu" } else { "cpu-vad-only" },
                    gpu_retried,
                    e,
                );
            }
            tracing::warn!(
                source,
                model = model_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                "whisper encode failed with VAD enabled, retrying without VAD"
            );
            result = self.run_inference(model_path, pcm, None, abort, Arc::clone(&on_progress));
            if result.is_ok() {
                tracing::info!(
                    source,
                    "transcription succeeded after disabling VAD fallback"
                );
            }
        }

        match result {
            Ok(segments) => {
                if segments.is_empty() {
                    tracing::debug!(
                        source,
                        pcm_samples = pcm.len(),
                        "whisper returned no segments (silence or skipped chunk)"
                    );
                }
                Ok(segments)
            }
            Err(InferError::Encode(e)) => {
                self.record_encode_failure(
                    source,
                    model_path,
                    pcm,
                    false,
                    !self.uses_gpu_for(model_path),
                    "no-vad",
                    vad_requested,
                    &e,
                );
                Err(e)
            }
            Err(e) => Err(e.into()),
        }
    }

    fn uses_gpu_for(&self, model_path: &Path) -> bool {
        !self
            .cpu_fallback_paths
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(model_path)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_encode_failure(
        &self,
        source: &str,
        model_path: &Path,
        pcm: &[f32],
        vad_enabled: bool,
        use_gpu: bool,
        attempt: &str,
        retried_from_gpu: bool,
        error: &anyhow::Error,
    ) {
        let diag = pcm_diagnostics(pcm);
        let model_integrity_ok = self.whisper_model_integrity_ok(model_path);
        let model_bytes = self.whisper_model_bytes(model_path);
        let model_sha256 = file_sha256_hex(model_path).ok();
        tracing::error!(
            source,
            attempt,
            retried_from_gpu,
            use_gpu,
            vad_enabled,
            model_integrity_ok,
            model_bytes,
            model = model_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            error = %format!("{error:#}"),
            pcm_samples = diag.samples,
            duration_secs = format!("{:.3}", diag.duration_secs),
            rms = format!("{:.6}", diag.rms),
            peak = format!("{:.6}", diag.peak),
            nan_count = diag.nan_count,
            inf_count = diag.inf_count,
            threads = inference_thread_count(),
            log_path = %self.failure_log_path.display(),
            "whisper encode failed"
        );
        self.persist_failure_record(TranscriptionFailureRecord {
            ts: chrono::Utc::now().to_rfc3339(),
            source: source.to_string(),
            attempt: attempt.to_string(),
            retried_from_gpu,
            use_gpu,
            vad_enabled,
            model: model_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| model_path.display().to_string()),
            model_bytes,
            model_integrity_ok,
            model_sha256,
            pcm_samples: diag.samples,
            duration_secs: diag.duration_secs,
            rms: diag.rms,
            peak: diag.peak,
            nan_count: diag.nan_count,
            inf_count: diag.inf_count,
            threads: inference_thread_count(),
            error: format!("{error:#}"),
        });
    }

    fn persist_failure_record(&self, record: TranscriptionFailureRecord) {
        let Ok(line) = serde_json::to_string(&record) else {
            return;
        };
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.failure_log_path)
        {
            let _ = writeln!(file, "{line}");
        }
    }

    fn run_inference<F>(
        &self,
        model_path: &Path,
        pcm: &[f32],
        vad_model_path: Option<&Path>,
        abort: Option<Arc<AtomicBool>>,
        on_progress: Arc<Mutex<F>>,
    ) -> Result<Vec<Segment>, InferError>
    where
        F: FnMut(f32) + Send + 'static,
    {
        let total_ms = ((pcm.len() as f32 / 16_000.0) * 1_000.0).max(1.0);
        let ctx = self
            .get_or_load_context(model_path)
            .map_err(InferError::Other)?;
        let mut state = ctx
            .create_state()
            .map_err(|e| InferError::Other(anyhow!("failed to create whisper state: {e:?}")))?;

        let n_threads = inference_thread_count();
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(n_threads as i32);
        params.set_language(Some("en"));
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_single_segment(false);
        if let Some(vad_path) = vad_model_path.and_then(|p| p.to_str()) {
            params.set_vad_model_path(Some(vad_path));
            params.enable_vad(true);
            params.set_vad_params(WhisperVadParams::default());
        }
        let prog = Arc::clone(&on_progress);
        params.set_segment_callback_safe_lossy(move |segment: SegmentCallbackData| {
            let p = progress_from_segment_end(segment.end_timestamp, total_ms);
            if let Ok(mut f) = prog.lock() {
                f(p);
            }
        });
        // Do not call `set_abort_callback_safe` — on whisper-rs 0.16 / Metal, registering
        // an abort callback (even one that always returns false) can fail encode with
        // GenericError(-6). Cooperative cancel is checked via `abort_flag` between retries.
        let _abort = abort;

        let infer_started = Instant::now();
        match state.full(params, pcm) {
            Ok(()) => {}
            Err(e @ (WhisperError::FailedToEncode | WhisperError::GenericError(_))) => {
                return Err(InferError::Encode(anyhow!(
                    "whisper inference failed: {e:?}"
                )));
            }
            Err(e) => {
                return Err(InferError::Other(anyhow!(
                    "whisper inference failed: {e:?}"
                )));
            }
        }
        let elapsed = infer_started.elapsed();
        let audio_secs = total_ms / 1000.0;
        let rtf = if elapsed.as_secs_f32() > 0.0 {
            audio_secs / elapsed.as_secs_f32()
        } else {
            f32::INFINITY
        };
        tracing::debug!(
            model = model_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".to_string()),
            audio_secs = format!("{audio_secs:.2}"),
            wall_secs = format!("{:.2}", elapsed.as_secs_f32()),
            rtf = format!("{rtf:.2}"),
            threads = n_threads,
            "transcription complete"
        );

        let mut segments = Vec::new();
        for seg in state.as_iter() {
            let text = seg.to_string();
            let text = text.trim().to_string();
            if !text.is_empty() {
                segments.push(Segment {
                    start_ms: seg.start_timestamp() * 10,
                    end_ms: seg.end_timestamp() * 10,
                    text,
                    source: None,
                });
            }
        }

        Ok(segments)
    }

    /// Merge dual-source segments chronologically with channel metadata.
    /// `SegmentSource::Mic` = local microphone; `SegmentSource::Speaker` = loopback/system audio.
    pub fn merge_dual_source(
        &self,
        mic_segments: &[Segment],
        speaker_segments: &[Segment],
    ) -> Vec<Segment> {
        #[derive(Clone)]
        struct Tagged {
            seg: Segment,
            is_speaker: bool,
        }

        let mut merged: Vec<Tagged> =
            Vec::with_capacity(mic_segments.len() + speaker_segments.len());
        merged.extend(mic_segments.iter().cloned().map(|seg| Tagged {
            seg,
            is_speaker: false,
        }));
        merged.extend(speaker_segments.iter().cloned().map(|seg| Tagged {
            seg,
            is_speaker: true,
        }));
        merged.sort_by_key(|item| item.seg.start_ms);

        let mut out: Vec<Segment> = Vec::with_capacity(merged.len());
        for item in merged {
            let text = item.seg.text.trim().to_string();
            let is_near_duplicate = out.last().is_some_and(|prev| {
                prev.text.ends_with(text.as_str())
                    && (item.seg.start_ms - prev.start_ms).abs() <= 1_500
            });
            if is_near_duplicate {
                continue;
            }
            out.push(Segment {
                start_ms: item.seg.start_ms,
                end_ms: item.seg.end_ms,
                text,
                source: Some(if item.is_speaker {
                    crate::types::SegmentSource::Speaker
                } else {
                    crate::types::SegmentSource::Mic
                }),
            });
        }
        out
    }
}

fn transcription_failure_log_path(models_dir: &Path) -> PathBuf {
    models_dir
        .parent()
        .map(|p| p.join("transcription-failures.jsonl"))
        .unwrap_or_else(|| models_dir.join("transcription-failures.jsonl"))
}

fn progress_from_segment_end(end_timestamp: i64, total_ms: f32) -> f32 {
    ((end_timestamp as f32 * 10.0) / total_ms).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy)]
struct PcmDiagnostics {
    samples: usize,
    duration_secs: f64,
    rms: f64,
    peak: f64,
    nan_count: u64,
    inf_count: u64,
}

fn pcm_diagnostics(pcm: &[f32]) -> PcmDiagnostics {
    let mut nan_count = 0u64;
    let mut inf_count = 0u64;
    let mut peak = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut valid = 0u64;
    for &s in pcm {
        if s.is_nan() {
            nan_count += 1;
            continue;
        }
        if s.is_infinite() {
            inf_count += 1;
            continue;
        }
        valid += 1;
        let abs = f64::from(s.abs());
        peak = peak.max(abs);
        sum_sq += f64::from(s) * f64::from(s);
    }
    let rms = if valid == 0 {
        0.0
    } else {
        (sum_sq / valid as f64).sqrt()
    };
    PcmDiagnostics {
        samples: pcm.len(),
        duration_secs: pcm.len() as f64 / 16_000.0,
        rms,
        peak,
        nan_count,
        inf_count,
    }
}

fn format_pcm_summary(pcm: &[f32]) -> String {
    let d = pcm_diagnostics(pcm);
    format!(
        "{} samples ({:.2}s), rms={:.4}, peak={:.4}, nan={}, inf={}",
        d.samples, d.duration_secs, d.rms, d.peak, d.nan_count, d.inf_count
    )
}

#[derive(serde::Serialize)]
struct TranscriptionFailureRecord {
    ts: String,
    source: String,
    attempt: String,
    retried_from_gpu: bool,
    use_gpu: bool,
    vad_enabled: bool,
    model: String,
    model_bytes: u64,
    model_integrity_ok: bool,
    model_sha256: Option<String>,
    pcm_samples: usize,
    duration_secs: f64,
    rms: f64,
    peak: f64,
    nan_count: u64,
    inf_count: u64,
    threads: usize,
    error: String,
}

fn inference_thread_count() -> usize {
    num_cpus::get_physical().clamp(1, MAX_INFERENCE_THREADS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_models_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liscribe-model-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn file_sha256_hex_matches_known_vector() {
        // SHA-256("abc") per FIPS 180-4 — pins both the digest and the hex encoding.
        let dir = temp_models_dir();
        let path = dir.join("abc.bin");
        std::fs::write(&path, b"abc").expect("write test file");
        assert_eq!(
            file_sha256_hex(&path).expect("hash file"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn model_path_for_known_id_is_stable() {
        let dir = temp_models_dir();
        let service = ModelService::new(dir.clone());
        assert_eq!(
            service.model_path_for_id("small-en-q5"),
            Some(dir.join(SMALL_MODEL_FILENAME))
        );
        assert!(service.model_path_for_id("unknown").is_none());
    }

    #[test]
    fn model_available_requires_non_empty_file() {
        let dir = temp_models_dir();
        let service = ModelService::new(dir.clone());
        let path = dir.join("ggml-tiny.bin");

        std::fs::write(&path, []).expect("write empty model file");
        assert!(!service.model_available(&path));

        std::fs::write(&path, [1, 2, 3]).expect("write non-empty model file");
        assert!(service.model_available(&path));
    }

    #[test]
    fn model_downloaded_reflects_disk_state_for_catalog_item() {
        let dir = temp_models_dir();
        let service = ModelService::new(dir.clone());
        let tiny_path = dir.join("ggml-tiny.en-q5_1.bin");

        assert!(!service.model_downloaded("tiny-en-q5"));
        std::fs::write(&tiny_path, [1]).expect("write tiny model");
        assert!(service.model_downloaded("tiny-en-q5"));
    }

    #[test]
    fn progress_from_segment_end_uses_audio_time_and_clamps() {
        assert_eq!(progress_from_segment_end(-10, 1_000.0), 0.0);
        assert_eq!(progress_from_segment_end(0, 1_000.0), 0.0);
        assert_eq!(progress_from_segment_end(50, 1_000.0), 0.5);
        assert_eq!(progress_from_segment_end(100, 1_000.0), 1.0);
        assert_eq!(progress_from_segment_end(150, 1_000.0), 1.0);
    }

    #[test]
    fn inference_thread_count_is_clamped() {
        let n = inference_thread_count();
        assert!(n >= 1, "thread count must be at least 1");
        assert!(
            n <= MAX_INFERENCE_THREADS,
            "thread count {n} exceeded MAX_INFERENCE_THREADS={MAX_INFERENCE_THREADS}"
        );
    }

    #[test]
    fn merge_dual_source_orders_and_labels_segments() {
        let service = ModelService::new(temp_models_dir());
        let mic = vec![Segment {
            start_ms: 2_000,
            end_ms: 2_500,
            text: "hello from mic".to_string(),
            source: None,
        }];
        let speaker = vec![Segment {
            start_ms: 1_000,
            end_ms: 1_500,
            text: "hello from speaker".to_string(),
            source: None,
        }];
        let merged = service.merge_dual_source(&mic, &speaker);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].source, Some(crate::types::SegmentSource::Speaker));
        assert_eq!(merged[1].source, Some(crate::types::SegmentSource::Mic));
        assert_eq!(merged[0].text, "hello from speaker");
        assert_eq!(merged[1].text, "hello from mic");
    }

    #[test]
    fn pcm_diagnostics_detects_nan_and_peak() {
        let mut pcm = vec![0.5f32; 16_000];
        pcm[0] = f32::NAN;
        pcm[1] = f32::INFINITY;
        let d = pcm_diagnostics(&pcm);
        assert_eq!(d.samples, 16_000);
        assert!((d.duration_secs - 1.0).abs() < f64::EPSILON);
        assert_eq!(d.nan_count, 1);
        assert_eq!(d.inf_count, 1);
        assert!((d.peak - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn transcription_failure_log_path_is_sibling_of_models_dir() {
        let models_dir = temp_models_dir().join("models");
        assert_eq!(
            transcription_failure_log_path(&models_dir),
            models_dir
                .parent()
                .unwrap()
                .join("transcription-failures.jsonl")
        );
    }

    #[test]
    fn vad_model_needs_redownload_when_corrupt() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        assert!(svc.vad_model_needs_redownload());

        let path = dir.join(VAD_MODEL_FILENAME);
        std::fs::write(&path, b"not a real vad model").unwrap();
        assert!(svc.vad_model_available());
        assert!(!svc.vad_model_integrity_ok());
        assert!(svc.vad_model_needs_redownload());
        assert!(svc.vad_path_for_inference().is_none());
    }

    #[test]
    fn vad_path_for_pcm_skips_short_clips() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join(VAD_MODEL_FILENAME);
        // Write bytes that won't match SHA but path exists — integrity fails, so inference path is None.
        std::fs::write(&path, b"stub").unwrap();
        assert!(svc.vad_path_for_pcm(16_000).is_none());
        assert!(svc.vad_path_for_pcm(32_000).is_none());
    }

    #[test]
    fn whisper_model_integrity_fails_when_catalog_hash_mismatch() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join("ggml-tiny.en-q5_1.bin");
        std::fs::write(&path, b"truncated or corrupt download").unwrap();
        assert!(svc.model_available(&path));
        assert!(!svc.whisper_model_integrity_ok(&path));
    }

    #[test]
    fn whisper_model_integrity_skips_non_catalog_paths() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join("custom-whisper.bin");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        assert!(svc.whisper_model_integrity_ok(&path));
    }

    #[test]
    #[ignore = "hardware: set SCRIBE_REGRESSION_WAV to a saved scribe mic.wav"]
    fn transcribe_saved_scribe_mic_wav_matches_dictate_path() {
        use crate::services::audio::read_wav_mono_f32;
        let wav = std::env::var("SCRIBE_REGRESSION_WAV")
            .map(PathBuf::from)
            .ok()
            .filter(|p| p.exists());
        let Some(wav) = wav else {
            eprintln!("skip: set SCRIBE_REGRESSION_WAV to a scribe session mic.wav");
            return;
        };
        let models_dir = std::env::var("SCRIBEFLOAT_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(
                    "/Users/benjamin/Library/Application Support/com.benjamin.scribefloat-v8/models",
                )
            });
        let model_path = models_dir.join("ggml-tiny.en-q5_1.bin");
        if !model_path.exists() {
            eprintln!("skip: tiny model not present at {}", model_path.display());
            return;
        }
        let svc = ModelService::new(models_dir);
        let pcm = read_wav_mono_f32(&wav).expect("read scribe mic.wav");
        eprintln!("pcm samples = {}", pcm.len());
        let result = svc.transcribe_pcm_with_progress(
            &model_path,
            &pcm,
            svc.vad_path_for_pcm(pcm.len()).as_deref(),
            None,
            "test/scribe-wav",
            |_| {},
        );
        eprintln!("result = {result:?}");
        result.expect("scribe session wav should transcribe via dictate-identical path");
    }

    #[test]
    fn reset_gpu_preference_clears_cpu_fallback_and_evicts_context() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir);
        let path = Path::new("/tmp/gpu_reset_test_model.bin");
        svc.mark_cpu_fallback(path);
        assert!(svc.cpu_fallback_paths.lock().unwrap().contains(path));
        svc.reset_gpu_preference(path);
        assert!(!svc.cpu_fallback_paths.lock().unwrap().contains(path));
        assert!(svc.lock_contexts().get(path).is_none());
    }

    #[test]
    fn mark_cpu_fallback_adds_to_set_and_evicts_context_cache() {
        let svc = ModelService::new(temp_models_dir());
        let path = Path::new("/tmp/nonexistent_model.bin");
        // Not in fallback set initially
        assert!(!svc.cpu_fallback_paths.lock().unwrap().contains(path));
        svc.mark_cpu_fallback(path);
        // Now marked for CPU-only loading
        assert!(svc.cpu_fallback_paths.lock().unwrap().contains(path));
        // Context cache has no entry (nothing was loaded, but eviction mustn't panic)
        assert!(svc.lock_contexts().get(path).is_none());
        // Idempotent
        svc.mark_cpu_fallback(path);
        assert!(svc.cpu_fallback_paths.lock().unwrap().contains(path));
    }
}
