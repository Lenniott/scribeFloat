use crate::types::{ModelDownloadEvent, Segment};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use whisper_rs::{
    FullParams, SamplingStrategy, SegmentCallbackData, WhisperContext, WhisperContextParameters,
    WhisperVadParams,
};

/// Cap inference threads at the number of physical cores. Hyperthreading does not help
/// matmul-heavy workloads and creates lock contention. The 8-thread upper bound matches
/// ggml's own scaling curve — adding threads beyond that on speech-length audio is a wash.
const MAX_INFERENCE_THREADS: usize = 8;

pub const SMALL_MODEL_FILENAME: &str = "ggml-small.en-q5_1.bin";

/// Model catalog ids eligible for record-start preload (small models only).
pub const PRELOAD_ELIGIBLE_MODEL_IDS: &[&str] = &["tiny-en-q5", "base-en-q5"];

pub fn model_id_preload_eligible(model_id: &str) -> bool {
    PRELOAD_ELIGIBLE_MODEL_IDS.contains(&model_id)
}

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
    /// the download is accepted unverified — fill these in (TODO(S1)) to enable pinning.
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
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
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

pub struct ModelService {
    models_dir: PathBuf,
    /// Loaded Whisper contexts keyed by canonical model path. A `WhisperContext` owns the
    /// model weights (~30 MB tiny → ~550 MB large turbo) and is safe to share across calls —
    /// only the per-inference `WhisperState` is created fresh on each transcribe. Caching
    /// here eliminates the cold-load tax (~300 ms tiny → ~2 s large) that the previous
    /// implementation paid on every `transcribe_pcm_with_progress` call.
    loaded_contexts: Mutex<HashMap<PathBuf, Arc<WhisperContext>>>,
    /// Per-path mutexes so concurrent callers (e.g. record-start preload + stop transcribe)
    /// serialize on the same file instead of each paying a full WhisperContext load.
    loading_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl ModelService {
    pub fn new(models_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            models_dir,
            loaded_contexts: Mutex::new(HashMap::new()),
            loading_locks: Mutex::new(HashMap::new()),
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

    /// Load a Whisper context for `model_path`, or return the cached one. Loads block on
    /// disk I/O and model parsing, so callers should invoke this from `spawn_blocking` (or
    /// off the async runtime). Subsequent calls for the same path are O(hash lookup).
    pub fn get_or_load_context(&self, model_path: &Path) -> Result<Arc<WhisperContext>> {
        if let Some(ctx) = self.cached_context(model_path) {
            return Ok(ctx);
        }

        let path_key = model_path.to_path_buf();
        let load_lock = self.load_lock_for(&path_key);
        let _in_flight = load_lock
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        if let Some(ctx) = self.cached_context(model_path) {
            return Ok(ctx);
        }

        let path_str = model_path
            .to_str()
            .ok_or_else(|| anyhow!("model path is not valid UTF-8"))?;
        let load_started = Instant::now();
        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
            .map_err(|e| anyhow!("failed to load model at {path_str}: {e:?}"))?;
        let ctx = Arc::new(ctx);
        eprintln!(
            "[model] loaded {} in {} ms",
            model_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path_str.to_string()),
            load_started.elapsed().as_millis()
        );

        let mut guard = self.lock_contexts();
        Ok(Arc::clone(
            guard
                .entry(path_key)
                .or_insert_with(|| Arc::clone(&ctx)),
        ))
    }

    fn cached_context(&self, model_path: &Path) -> Option<Arc<WhisperContext>> {
        let guard = self.lock_contexts();
        guard.get(model_path).map(Arc::clone)
    }

    fn load_lock_for(&self, path_key: &Path) -> Arc<Mutex<()>> {
        let mut locks = self
            .loading_locks
            .lock()
            .unwrap_or_else(|p| p.into_inner());
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

    /// Transcribe mono f32 PCM at 16 kHz and report Whisper's own progress.
    /// Must be called from spawn_blocking.
    /// Pass `vad_model_path` to enable Silero VAD — silence mid-recording is skipped,
    /// preventing hallucinations during pauses.
    pub fn transcribe_pcm_with_progress<F>(
        &self,
        model_path: &Path,
        pcm: &[f32],
        vad_model_path: Option<&Path>,
        abort: Option<Arc<AtomicBool>>,
        mut on_progress: F,
    ) -> Result<Vec<Segment>>
    where
        F: FnMut(f32) + 'static,
    {
        let total_ms = ((pcm.len() as f32 / 16_000.0) * 1_000.0).max(1.0);
        let ctx = self.get_or_load_context(model_path)?;
        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow!("failed to create whisper state: {e:?}"))?;

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
        params.set_segment_callback_safe_lossy(move |segment: SegmentCallbackData| {
            on_progress(progress_from_segment_end(segment.end_timestamp, total_ms));
        });
        // Let an in-flight inference be interrupted: whisper.cpp polls this between work units
        // and bails out of `full()` early when it returns true. Without this, an abort is only
        // observed after the entire pass completes.
        if let Some(abort) = abort {
            params.set_abort_callback_safe(move || abort.load(Ordering::SeqCst));
        }

        let infer_started = Instant::now();
        state
            .full(params, pcm)
            .map_err(|e| anyhow!("whisper inference failed: {e:?}"))?;
        let elapsed = infer_started.elapsed();
        let audio_secs = total_ms / 1000.0;
        let rtf = if elapsed.as_secs_f32() > 0.0 {
            audio_secs / elapsed.as_secs_f32()
        } else {
            f32::INFINITY
        };
        eprintln!(
            "[transcribe] model={} audio={:.2}s wall={:.2}s rtf={:.2}x threads={}",
            model_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".to_string()),
            audio_secs,
            elapsed.as_secs_f32(),
            rtf,
            n_threads,
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
                });
            }
        }

        Ok(segments)
    }

    /// Merge dual-source segments chronologically and label channel origin.
    /// `in:` = speaker/system audio, `out:` = local microphone.
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

        let mut merged: Vec<Tagged> = Vec::with_capacity(mic_segments.len() + speaker_segments.len());
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
            // Product semantics:
            // in: local microphone (what I say)
            // out: speaker/system audio (what I hear)
            let label = if item.is_speaker { "out:" } else { "in:" };
            let text = format!("{label} {}", item.seg.text.trim());
            // Suppress likely mic bleed duplicates when same text appears very close in time.
            let is_near_duplicate = out.last().is_some_and(|prev| {
                prev.text.ends_with(item.seg.text.trim())
                    && (item.seg.start_ms - prev.start_ms).abs() <= 1_500
            });
            if is_near_duplicate {
                continue;
            }
            out.push(Segment {
                start_ms: item.seg.start_ms,
                end_ms: item.seg.end_ms,
                text,
            });
        }
        out
    }
}

fn progress_from_segment_end(end_timestamp: i64, total_ms: f32) -> f32 {
    ((end_timestamp as f32 * 10.0) / total_ms).clamp(0.0, 1.0)
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
    fn model_id_preload_eligible_only_tiny_and_base() {
        assert!(model_id_preload_eligible("tiny-en-q5"));
        assert!(model_id_preload_eligible("base-en-q5"));
        assert!(!model_id_preload_eligible("small-en-q5"));
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
        }];
        let speaker = vec![Segment {
            start_ms: 1_000,
            end_ms: 1_500,
            text: "hello from speaker".to_string(),
        }];
        let merged = service.merge_dual_source(&mic, &speaker);
        assert_eq!(merged.len(), 2);
        // Sorted by start_ms: speaker (1000ms) before mic (2000ms).
        assert!(merged[0].text.starts_with("out: "));
        assert!(merged[1].text.starts_with("in: "));
    }
}
