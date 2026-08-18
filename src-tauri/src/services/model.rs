use crate::services::bundled_models::{self, file_sha256_hex};
use crate::types::Segment;
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperError,
    WhisperVadParams,
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

/// Provenance label for the single Whisper model shipped with the app
/// (see scripts/fetch-bundled-models.sh). Not a chooser id.
pub const DEFAULT_MODEL_ID: &str = "small-en-q5";

/// Verified lowercase-hex SHA-256 of the bundled Whisper Small file.
pub const SMALL_MODEL_SHA256: &str =
    "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30";

pub const VAD_MODEL_FILENAME: &str = "ggml-silero-v6.2.0.bin";

/// Verified lowercase-hex SHA-256 of the bundled Silero VAD file.
pub const VAD_MODEL_SHA256: &str =
    "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987";

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
    /// Installed app resource dir (signed bundle models). Used to offline-restore
    /// a bad writable copy. `None` in unit tests.
    resource_dir: Option<PathBuf>,
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
    /// SHA-256-verified model files, stamped with (mtime, len). Hashing a model is
    /// hundreds of ms (181 MB small) to seconds (large) — without this cache every
    /// transcription paid it under the "loading model" phase. A changed stamp
    /// (re-download, manual replacement) re-hashes automatically.
    verified_models: Mutex<HashMap<PathBuf, (std::time::SystemTime, u64)>>,
}

impl ModelService {
    pub fn new(models_dir: PathBuf) -> Arc<Self> {
        Self::with_resource_dir(models_dir, None)
    }

    pub fn with_resource_dir(models_dir: PathBuf, resource_dir: Option<PathBuf>) -> Arc<Self> {
        let failure_log_path = transcription_failure_log_path(&models_dir);
        Arc::new(Self {
            models_dir,
            resource_dir,
            failure_log_path,
            loaded_contexts: Mutex::new(HashMap::new()),
            loading_locks: Mutex::new(HashMap::new()),
            cpu_fallback_paths: Mutex::new(HashSet::new()),
            inference_gate: Mutex::new(()),
            verified_models: Mutex::new(HashMap::new()),
        })
    }

    /// Path where the bundled Small Whisper model lives on disk.
    pub fn default_model_path(&self) -> PathBuf {
        self.models_dir.join(SMALL_MODEL_FILENAME)
    }

    pub fn model_available(&self, path: &Path) -> bool {
        path.exists()
            && std::fs::metadata(path)
                .map(|m| m.len() > 0)
                .unwrap_or(false)
    }

    /// Whether the bundled Whisper Small file is present and non-empty.
    pub fn bundled_model_available(&self) -> bool {
        self.model_available(&self.default_model_path())
    }

    pub fn vad_model_path(&self) -> PathBuf {
        self.models_dir.join(VAD_MODEL_FILENAME)
    }

    pub fn vad_model_available(&self) -> bool {
        self.model_available(&self.vad_model_path())
    }

    /// Whether the on-disk VAD file matches the bundled SHA-256.
    pub fn vad_model_integrity_ok(&self) -> bool {
        let path = self.vad_model_path();
        if !self.vad_model_available() {
            return false;
        }
        file_sha256_hex(&path)
            .map(|actual| actual == VAD_MODEL_SHA256.to_ascii_lowercase())
            .unwrap_or(false)
    }

    /// Whether the bundled VAD file is present and passes SHA-256.
    pub fn bundled_vad_available(&self) -> bool {
        self.ensure_vad_integrity()
    }

    /// Offline-restore VAD from the signed app bundle when the writable copy is bad.
    fn ensure_vad_integrity(&self) -> bool {
        let path = self.vad_model_path();
        bundled_models::ensure_bundled_file(
            self.resource_dir.as_deref(),
            &path,
            VAD_MODEL_FILENAME,
            VAD_MODEL_SHA256,
        )
    }

    /// Offline-restore Whisper Small from the signed app bundle when the writable copy is bad.
    fn ensure_whisper_integrity(&self, model_path: &Path) -> bool {
        let Some(expected) = self.bundled_sha256_for_path(model_path) else {
            return self.model_available(model_path);
        };
        let Some(name) = model_path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };
        // Drop any stale "verified" stamp before/after restore so a healed file re-hashes.
        self.lock_verified_models().remove(model_path);
        let ok = bundled_models::ensure_bundled_file(
            self.resource_dir.as_deref(),
            model_path,
            name,
            expected,
        );
        if ok {
            let _ = self.model_integrity_ok_cached(model_path, expected);
        }
        ok
    }

    /// Resolve VAD for a PCM length. Short clips skip VAD (encoder constraint).
    /// Longer clips require the bundled VAD — missing/corrupt → clear offline error.
    pub fn vad_path_for_pcm(&self, pcm_samples: usize) -> Result<Option<PathBuf>> {
        if pcm_samples < VAD_MIN_PCM_SAMPLES {
            return Ok(None);
        }
        if !self.ensure_vad_integrity() {
            if !self.vad_model_available() {
                return Err(anyhow!(
                    "Bundled voice-activity model missing. Reinstall the app to restore bundled models."
                ));
            }
            return Err(anyhow!(
                "Bundled voice-activity model failed integrity check. Reinstall the app to restore bundled models."
            ));
        }
        Ok(Some(self.vad_model_path()))
    }

    /// Whether the bundled Whisper file on disk matches its published SHA-256.
    /// Non-bundled filenames skip verification and only require a non-empty file.
    pub fn whisper_model_integrity_ok(&self, model_path: &Path) -> bool {
        if !self.model_available(model_path) {
            return false;
        }
        let Some(expected) = self.bundled_sha256_for_path(model_path) else {
            return true;
        };
        file_sha256_hex(model_path)
            .map(|actual| actual == expected.to_ascii_lowercase())
            .unwrap_or(false)
    }

    /// Integrity check with a session cache: hash once, then trust the file
    /// identity (mtime + len). The stamp — not the `expected` argument — is the
    /// cache key, so a changed file always re-hashes and a failed hash is never
    /// remembered. Use on hot paths; `whisper_model_integrity_ok` stays uncached
    /// for diagnostics.
    pub fn model_integrity_ok_cached(&self, model_path: &Path, expected: &str) -> bool {
        let Ok(meta) = std::fs::metadata(model_path) else {
            return false;
        };
        let Ok(modified) = meta.modified() else {
            // No usable stamp on this filesystem — fall back to hashing every time.
            return self.hash_matches(model_path, expected);
        };
        let stamp = (modified, meta.len());
        {
            let cache = self.lock_verified_models();
            if cache.get(model_path) == Some(&stamp) {
                return true;
            }
        }
        let ok = self.hash_matches(model_path, expected);
        if ok {
            self.lock_verified_models()
                .insert(model_path.to_path_buf(), stamp);
        }
        ok
    }

    fn hash_matches(&self, model_path: &Path, expected: &str) -> bool {
        file_sha256_hex(model_path)
            .map(|actual| actual == expected.to_ascii_lowercase())
            .unwrap_or(false)
    }

    fn lock_verified_models(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<PathBuf, (std::time::SystemTime, u64)>> {
        self.verified_models
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    pub fn whisper_model_bytes(&self, model_path: &Path) -> u64 {
        std::fs::metadata(model_path).map(|m| m.len()).unwrap_or(0)
    }

    fn bundled_sha256_for_path(&self, model_path: &Path) -> Option<&'static str> {
        let name = model_path.file_name()?.to_str()?;
        (name == SMALL_MODEL_FILENAME).then_some(SMALL_MODEL_SHA256)
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

    /// Bring the Whisper context for `model_path` fully to ready (disk parse plus
    /// GPU/CPU backend setup) so the next transcription starts as a cache hit.
    /// Shares the per-path load lock with inference: a stop-and-transcribe that
    /// lands mid-preload waits for this load instead of duplicating it. Failures
    /// are logged and deferred — transcription will retry the load and surface
    /// the error to the user. Blocking; call from a background thread.
    pub fn preload_context(&self, model_path: &Path) {
        if !self.model_available(model_path) {
            return;
        }
        // Pay the SHA-256 integrity hash here too, so stop-and-transcribe finds
        // both the context and the verification already warm.
        if let Some(expected) = self.bundled_sha256_for_path(model_path) {
            let _ = self.model_integrity_ok_cached(model_path, expected);
        }
        if let Err(err) = self.get_or_load_context(model_path) {
            tracing::warn!(
                error = %err,
                "whisper preload failed; model will load at transcription time"
            );
        }
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
    /// retries GPU. Only when the flag was actually set is the cached context
    /// evicted (it was loaded with `use_gpu = false` and must not be reused) —
    /// a healthy cached context survives, so preloads and prior transcriptions
    /// keep paying off. Returns whether a context was evicted.
    fn reset_gpu_preference(&self, model_path: &Path) -> bool {
        let was_cpu_fallback = self
            .cpu_fallback_paths
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(model_path);
        if was_cpu_fallback {
            self.evict_context(model_path);
        }
        was_cpu_fallback
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
    #[allow(clippy::too_many_arguments)]
    pub fn transcribe_pcm_with_progress<F>(
        &self,
        model_path: &Path,
        pcm: &[f32],
        vad_model_path: Option<&Path>,
        abort: Option<Arc<AtomicBool>>,
        source: &str,
        on_progress: F,
        on_model_loaded: Option<Box<dyn FnOnce() + Send>>,
    ) -> Result<Vec<Segment>>
    where
        F: FnMut(f32) + Send + 'static,
    {
        let _inference = self
            .inference_gate
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        self.reset_gpu_preference(model_path);

        if let Some(expected) = self.bundled_sha256_for_path(model_path) {
            if !self.model_integrity_ok_cached(model_path, expected)
                && !self.ensure_whisper_integrity(model_path)
            {
                let actual = file_sha256_hex(model_path).unwrap_or_default();
                return Err(anyhow!(
                    "Whisper model failed SHA-256 integrity check (expected {expected}, got {actual}). Reinstall the app to restore the bundled model."
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
            on_model_loaded,
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
                None,
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
            result =
                self.run_inference(model_path, pcm, None, abort, None, Arc::clone(&on_progress));
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
        on_model_loaded: Option<Box<dyn FnOnce() + Send>>,
        on_progress: Arc<Mutex<F>>,
    ) -> Result<Vec<Segment>, InferError>
    where
        F: FnMut(f32) + Send + 'static,
    {
        let total_ms = ((pcm.len() as f32 / 16_000.0) * 1_000.0).max(1.0);
        let ctx = self
            .get_or_load_context(model_path)
            .map_err(InferError::Other)?;
        if let Some(cb) = on_model_loaded {
            cb();
        }
        if let Ok(mut f) = on_progress.lock() {
            f(INFERENCE_MODEL_LOAD_PROGRESS);
        }
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
        let throttle = Arc::new(Mutex::new(WhisperProgressThrottle::new()));
        let throttle_arc = Arc::clone(&throttle);
        params.set_progress_callback_safe(move |percent| {
            let mut guard = match throttle_arc.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            if !guard.should_emit(percent) {
                return;
            }
            guard.record(percent);
            let p = inference_progress_from_whisper_percent(percent);
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
                    speaker: None,
                });
            }
        }

        Self::log_segment_granularity(&segments);

        Ok(segments)
    }

    /// Baseline diagnostic for the segment-coarseness question: how many
    /// segments Whisper produced and how long they are. Compare against the
    /// same numbers after enabling token-level timestamps to see whether
    /// finer segmentation is worth the alignment/rendering churn it costs.
    fn log_segment_granularity(segments: &[Segment]) {
        if segments.is_empty() {
            return;
        }
        let durations_ms: Vec<i64> = segments
            .iter()
            .map(|s| (s.end_ms - s.start_ms).max(0))
            .collect();
        let total_ms: i64 = durations_ms.iter().sum();
        let avg_ms = total_ms / durations_ms.len() as i64;
        let min_ms = durations_ms.iter().min().copied().unwrap_or(0);
        let max_ms = durations_ms.iter().max().copied().unwrap_or(0);
        tracing::info!(
            segment_count = segments.len(),
            avg_segment_ms = avg_ms,
            min_segment_ms = min_ms,
            max_segment_ms = max_ms,
            "ASR segment granularity (baseline: whisper segment-level timestamps)"
        );
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
                speaker: None,
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

fn progress_from_whisper_percent(percent: i32) -> f32 {
    (percent as f32 / 100.0).clamp(0.0, 1.0)
}

/// Reserve the first slice of the 0–1 bar for model load; Whisper's callback covers encode+decode.
pub const INFERENCE_MODEL_LOAD_PROGRESS: f32 = 0.05;

fn inference_progress_from_whisper_percent(percent: i32) -> f32 {
    INFERENCE_MODEL_LOAD_PROGRESS
        + progress_from_whisper_percent(percent) * (1.0 - INFERENCE_MODEL_LOAD_PROGRESS)
}

struct WhisperProgressThrottle {
    last_percent: Option<i32>,
    last_emit: Instant,
}

impl WhisperProgressThrottle {
    fn new() -> Self {
        Self {
            last_percent: None,
            last_emit: Instant::now(),
        }
    }

    fn should_emit(&self, percent: i32) -> bool {
        should_emit_whisper_progress(self.last_percent, self.last_emit.elapsed(), percent)
    }

    fn record(&mut self, percent: i32) {
        self.last_percent = Some(percent);
        self.last_emit = Instant::now();
    }
}

fn should_emit_whisper_progress(
    last_percent: Option<i32>,
    since_last_emit: Duration,
    percent: i32,
) -> bool {
    if percent >= 100 {
        return true;
    }
    if last_percent.is_none() {
        return true;
    }
    if let Some(last) = last_percent {
        if (percent - last).abs() >= 1 {
            return true;
        }
    }
    since_last_emit >= Duration::from_millis(100)
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
    fn default_model_path_is_bundled_filename() {
        let dir = temp_models_dir();
        let service = ModelService::new(dir.clone());
        assert_eq!(service.default_model_path(), dir.join(SMALL_MODEL_FILENAME));
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
    fn bundled_model_available_reflects_disk_state() {
        let dir = temp_models_dir();
        let service = ModelService::new(dir.clone());
        let small_path = dir.join(SMALL_MODEL_FILENAME);

        assert!(!service.bundled_model_available());
        std::fs::write(&small_path, [1]).expect("write small model");
        assert!(service.bundled_model_available());
    }

    #[test]
    fn progress_from_whisper_percent_maps_0_to_100() {
        assert_eq!(progress_from_whisper_percent(-10), 0.0);
        assert_eq!(progress_from_whisper_percent(0), 0.0);
        assert_eq!(progress_from_whisper_percent(50), 0.5);
        assert_eq!(progress_from_whisper_percent(100), 1.0);
        assert_eq!(progress_from_whisper_percent(150), 1.0);
    }

    #[test]
    fn inference_progress_reserves_headroom_for_model_load() {
        assert_eq!(
            inference_progress_from_whisper_percent(0),
            INFERENCE_MODEL_LOAD_PROGRESS
        );
        assert_eq!(inference_progress_from_whisper_percent(100), 1.0);
    }

    #[test]
    fn whisper_progress_throttle_emits_on_first_percent_change_or_10hz() {
        assert!(should_emit_whisper_progress(
            None,
            Duration::from_millis(0),
            0
        ));
        assert!(!should_emit_whisper_progress(
            Some(5),
            Duration::from_millis(50),
            5
        ));
        assert!(should_emit_whisper_progress(
            Some(5),
            Duration::from_millis(50),
            6
        ));
        assert!(should_emit_whisper_progress(
            Some(6),
            Duration::from_millis(100),
            6
        ));
        assert!(should_emit_whisper_progress(
            Some(99),
            Duration::from_millis(0),
            100
        ));
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
            speaker: None,
        }];
        let speaker = vec![Segment {
            start_ms: 1_000,
            end_ms: 1_500,
            text: "hello from speaker".to_string(),
            source: None,
            speaker: None,
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
    fn bundled_vad_unavailable_when_corrupt() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        assert!(!svc.bundled_vad_available());

        let path = dir.join(VAD_MODEL_FILENAME);
        std::fs::write(&path, b"not a real vad model").unwrap();
        assert!(svc.vad_model_available());
        assert!(!svc.vad_model_integrity_ok());
        assert!(!svc.bundled_vad_available());
        let err = svc.vad_path_for_pcm(32_000).unwrap_err().to_string();
        assert!(err.contains("integrity check"));
        assert!(err.contains("Reinstall"));
    }

    #[test]
    fn vad_path_for_pcm_skips_short_clips_even_when_missing() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir);
        assert!(svc.vad_path_for_pcm(16_000).unwrap().is_none());
        let err = svc.vad_path_for_pcm(32_000).unwrap_err().to_string();
        assert!(err.contains("missing"));
        assert!(err.contains("Reinstall"));
    }

    #[test]
    fn whisper_model_integrity_fails_when_catalog_hash_mismatch() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join(SMALL_MODEL_FILENAME);
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
    fn verified_model_is_not_rehashed_while_unchanged() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join("model.bin");
        std::fs::write(&path, b"model bytes").unwrap();
        let sha = file_sha256_hex(&path).unwrap();
        assert!(svc.model_integrity_ok_cached(&path, &sha));
        // The cache trusts the unchanged file identity (mtime + len); a second
        // call must not re-hash — observable because a bogus expected hash
        // still passes while the file is unchanged.
        assert!(svc.model_integrity_ok_cached(&path, "not-a-real-hash"));
    }

    #[test]
    fn changing_the_file_invalidates_the_verification_cache() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join("model.bin");
        std::fs::write(&path, b"model bytes").unwrap();
        let sha = file_sha256_hex(&path).unwrap();
        assert!(svc.model_integrity_ok_cached(&path, &sha));
        std::fs::write(&path, b"different bytes, different length").unwrap();
        assert!(!svc.model_integrity_ok_cached(&path, &sha));
    }

    #[test]
    fn failed_verification_is_not_cached() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        let path = dir.join("model.bin");
        std::fs::write(&path, b"model bytes").unwrap();
        let sha = file_sha256_hex(&path).unwrap();
        assert!(!svc.model_integrity_ok_cached(&path, "wrong"));
        assert!(svc.model_integrity_ok_cached(&path, &sha));
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
        let model_path = models_dir.join(SMALL_MODEL_FILENAME);
        if !model_path.exists() {
            eprintln!("skip: small model not present at {}", model_path.display());
            return;
        }
        let svc = ModelService::new(models_dir);
        let pcm = read_wav_mono_f32(&wav).expect("read scribe mic.wav");
        eprintln!("pcm samples = {}", pcm.len());
        let vad = svc.vad_path_for_pcm(pcm.len()).expect("resolve VAD for test wav");
        let result = svc.transcribe_pcm_with_progress(
            &model_path,
            &pcm,
            vad.as_deref(),
            None,
            "test/scribe-wav",
            |_| {},
            None,
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
    fn reset_gpu_preference_keeps_context_for_healthy_path() {
        let svc = ModelService::new(temp_models_dir());
        let path = Path::new("/tmp/healthy_model.bin");
        // Never marked for CPU fallback → nothing to reset; a context cached by a
        // preload or a prior transcription must survive.
        assert!(!svc.reset_gpu_preference(path));
        svc.mark_cpu_fallback(path);
        assert!(svc.reset_gpu_preference(path));
    }

    #[test]
    fn preload_context_handles_missing_and_corrupt_models_without_panicking() {
        let dir = temp_models_dir();
        let svc = ModelService::new(dir.clone());
        // Missing file: silently skipped, nothing cached.
        svc.preload_context(Path::new("/tmp/definitely-missing-model.bin"));
        // Corrupt file: load fails, warns, nothing cached.
        let corrupt = dir.join("corrupt.bin");
        std::fs::write(&corrupt, b"not a ggml model").unwrap();
        svc.preload_context(&corrupt);
        assert!(svc.lock_contexts().is_empty());
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
