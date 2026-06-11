use crate::services::config::ConfigService;
use crate::services::history::HistoryService;
use crate::services::model::ModelService;
use crate::services::output::OutputService;
use crate::services::transcribe_input::{TranscribeInputItem, TranscribeInputService};
use crate::types::{
    Config, HistoryRecord, ProcessingStage, TranscribeItemStatus, TranscribeQueueItem,
    TranscribeState, TranscribeStateEvent,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

struct Inner {
    state: TranscribeState,
}

pub struct TranscribeStartRequest {
    pub input_paths: Vec<String>,
    pub output_folder: Option<String>,
    pub model_id: Option<String>,
    pub include_timestamps: Option<bool>,
}

pub struct TranscribeController {
    inner: Mutex<Inner>,
    input: Arc<TranscribeInputService>,
    model: Arc<ModelService>,
    output: Arc<OutputService>,
    history: Arc<HistoryService>,
    config: Arc<ConfigService>,
    app: AppHandle,
}

impl TranscribeController {
    pub fn new(
        input: Arc<TranscribeInputService>,
        model: Arc<ModelService>,
        output: Arc<OutputService>,
        history: Arc<HistoryService>,
        config: Arc<ConfigService>,
        app: AppHandle,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                state: TranscribeState::Idle,
            }),
            input,
            model,
            output,
            history,
            config,
            app,
        })
    }

    pub fn inspect_inputs(
        &self,
        input_paths: Vec<String>,
    ) -> Result<Vec<TranscribeQueueItem>, String> {
        let items = self.input.expand_inputs(&input_paths)?;
        Ok(items
            .into_iter()
            .map(|item| TranscribeQueueItem {
                id: item.id,
                source_path: item.source_path.to_string_lossy().to_string(),
                display_name: item.display_name,
                source_type: item.source_type,
                duration_ms: item.duration_ms,
                status: TranscribeItemStatus::Queued,
                progress: 0.0,
                transcript_path: None,
                error: None,
            })
            .collect())
    }

    pub fn start(this: Arc<Self>, request: TranscribeStartRequest) -> Result<(), String> {
        let cfg = this.config.get();
        let inputs = this.input.expand_inputs(&request.input_paths)?;
        let output_folder =
            resolve_output_folder(&this.output, &cfg, request.output_folder.as_deref())?;
        let include_timestamps = request.include_timestamps.unwrap_or(cfg.include_timestamps);
        let model_path = resolve_model_path(&cfg, this.model.as_ref(), request.model_id.as_deref());
        if !this.model.model_available(&model_path) {
            return Err("selected model is not downloaded".to_string());
        }
        {
            let mut inner = this.lock();
            if inner.state == TranscribeState::Transcribing {
                return Err("transcribe is already running".to_string());
            }
            inner.state = TranscribeState::Transcribing;
        }

        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().replace("ggml-", ""))
            .unwrap_or_else(|| "model".to_string());
        let mut queue = build_queue_items(&inputs);
        this.emit_queue_state(
            TranscribeState::Transcribing,
            queue.clone(),
            Some(0.0),
            Some(ProcessingStage::LoadingModel),
            None,
        );

        let replacement_rules = cfg.replacement_rules.clone();
        let replacement_prefix = cfg.replacement_prefix.clone();
        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result = tokio::task::spawn_blocking(move || {
                ctrl.run_batch(
                    inputs,
                    &model_path,
                    &model_name,
                    &output_folder,
                    include_timestamps,
                    &replacement_rules,
                    &replacement_prefix,
                    &mut queue,
                )
            })
            .await;

            match result {
                Ok(Ok(final_queue)) => {
                    this.lock().state = TranscribeState::Done;
                    this.emit_queue_state(
                        TranscribeState::Done,
                        final_queue,
                        Some(1.0),
                        None,
                        None,
                    );
                }
                Ok(Err(err)) => {
                    this.lock().state = TranscribeState::Error;
                    this.emit_queue_state(
                        TranscribeState::Error,
                        Vec::new(),
                        None,
                        None,
                        Some(err),
                    );
                }
                Err(err) => {
                    this.lock().state = TranscribeState::Error;
                    this.emit_queue_state(
                        TranscribeState::Error,
                        Vec::new(),
                        None,
                        None,
                        Some(format!("transcribe task failed: {err}")),
                    );
                }
            }
        });

        Ok(())
    }

    pub fn open_output_path(&self, path: &str) -> Result<(), String> {
        let canonical = std::fs::canonicalize(Path::new(path))
            .map_err(|e| format!("failed to resolve output path `{path}`: {e}"))?;
        if !canonical.is_file() {
            return Err(format!("`{}` is not a file", canonical.display()));
        }
        // Transcribe only ever writes `.md` transcripts. Restrict the OS "open" hand-off to that
        // extension so this command can't be coerced into launching an arbitrary file type
        // (e.g. an executable or `.app`) via the default handler.
        let is_markdown = canonical
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if !is_markdown {
            return Err("only transcript (.md) files can be opened".to_string());
        }
        let open_with = self.config.get().open_with_app_path;
        self.output
            .open_file_for_user(canonical.to_string_lossy().as_ref(), open_with.as_deref())
    }

    #[allow(clippy::too_many_arguments)]
    fn run_batch(
        &self,
        inputs: Vec<TranscribeInputItem>,
        model_path: &Path,
        model_name: &str,
        output_folder: &Path,
        include_timestamps: bool,
        replacement_rules: &[crate::types::ReplacementRule],
        replacement_prefix: &str,
        queue: &mut [TranscribeQueueItem],
    ) -> Result<Vec<TranscribeQueueItem>, String> {
        // Snapshot once: history records always go to the save folder; `.md` is opt-in (and may
        // target a different output folder).
        let cfg = self.config.get();
        let save_folder = cfg.save_folder.clone();
        let markdown_on = cfg.save_transcripts_as_markdown;
        for (index, input) in inputs.iter().enumerate() {
            if queue.get(index).is_none() {
                continue;
            }
            queue[index].status = TranscribeItemStatus::Processing;
            queue[index].progress = 0.0;
            queue[index].error = None;
            self.emit_queue_state(
                TranscribeState::Transcribing,
                queue.to_vec(),
                Some(overall_progress(queue)),
                Some(ProcessingStage::LoadingModel),
                None,
            );

            let decoded = match self.input.decode_input(input) {
                Ok(decoded) => decoded,
                Err(err) => {
                    queue[index].status = TranscribeItemStatus::Error;
                    queue[index].error = Some(err);
                    queue[index].progress = 1.0;
                    self.emit_queue_state(
                        TranscribeState::Transcribing,
                        queue.to_vec(),
                        Some(overall_progress(queue)),
                        Some(ProcessingStage::TranscribingAudio),
                        None,
                    );
                    continue;
                }
            };

            let vad_path = self.model.vad_model_path();
            let vad = self
                .model
                .model_available(&vad_path)
                .then_some(vad_path.as_path());
            let segments = if let Some(speaker_pcm) = decoded.speaker_pcm_16k.as_ref() {
                let mic_segments = self
                    .model
                    .transcribe_pcm_with_progress(
                        model_path,
                        &decoded.mic_pcm_16k,
                        vad,
                        None,
                        "transcribe/mic",
                        {
                            let app = self.app.clone();
                            let item_id = queue[index].id.clone();
                            move |p| {
                                let _ = app.emit(
                                    "transcribe://item-progress",
                                    serde_json::json!({
                                        "item_id": item_id,
                                        "progress": p * 0.5
                                    }),
                                );
                            }
                        },
                    )
                    .map_err(|e| e.to_string());
                let mic_segments = match mic_segments {
                    Ok(segments) => segments,
                    Err(err) => {
                        queue[index].status = TranscribeItemStatus::Error;
                        queue[index].error = Some(err);
                        queue[index].progress = 1.0;
                        self.emit_queue_state(
                            TranscribeState::Transcribing,
                            queue.to_vec(),
                            Some(overall_progress(queue)),
                            Some(ProcessingStage::TranscribingAudio),
                            None,
                        );
                        continue;
                    }
                };

                let speaker_segments = self
                    .model
                    .transcribe_pcm_with_progress(
                        model_path,
                        speaker_pcm,
                        vad,
                        None,
                        "transcribe/speaker",
                        {
                            let app = self.app.clone();
                            let item_id = queue[index].id.clone();
                            move |p| {
                                let _ = app.emit(
                                    "transcribe://item-progress",
                                    serde_json::json!({
                                        "item_id": item_id,
                                        "progress": 0.5 + p * 0.5
                                    }),
                                );
                            }
                        },
                    )
                    .map_err(|e| e.to_string());
                let speaker_segments = match speaker_segments {
                    Ok(segments) => segments,
                    Err(err) => {
                        queue[index].status = TranscribeItemStatus::Error;
                        queue[index].error = Some(err);
                        queue[index].progress = 1.0;
                        self.emit_queue_state(
                            TranscribeState::Transcribing,
                            queue.to_vec(),
                            Some(overall_progress(queue)),
                            Some(ProcessingStage::TranscribingAudio),
                            None,
                        );
                        continue;
                    }
                };

                self.model
                    .merge_dual_source(&mic_segments, &speaker_segments)
            } else {
                match self.model.transcribe_pcm_with_progress(
                    model_path,
                    &decoded.mic_pcm_16k,
                    vad,
                    None,
                    "transcribe/mic",
                    {
                        let app = self.app.clone();
                        let item_id = queue[index].id.clone();
                        move |p| {
                            let _ = app.emit(
                                "transcribe://item-progress",
                                serde_json::json!({
                                    "item_id": item_id,
                                    "progress": p
                                }),
                            );
                        }
                    },
                ) {
                    Ok(segments) => segments,
                    Err(err) => {
                        queue[index].status = TranscribeItemStatus::Error;
                        queue[index].error = Some(err.to_string());
                        queue[index].progress = 1.0;
                        self.emit_queue_state(
                            TranscribeState::Transcribing,
                            queue.to_vec(),
                            Some(overall_progress(queue)),
                            Some(ProcessingStage::TranscribingAudio),
                            None,
                        );
                        continue;
                    }
                }
            };

            queue[index].progress = 0.98;
            self.emit_queue_state(
                TranscribeState::Transcribing,
                queue.to_vec(),
                Some(overall_progress(queue)),
                Some(ProcessingStage::WritingTranscript),
                None,
            );

            let dual_source = decoded.speaker_pcm_16k.is_some();
            // Markdown is opt-in; write `.md` only when the toggle is on.
            let markdown_path = if markdown_on {
                let output_name = format!("{}_{}.md", slugify(&input.display_name), model_name);
                let transcript_dest = output_folder.join(output_name);
                match self.output.write_transcript(
                    &segments,
                    &[],
                    &input.display_name,
                    model_name,
                    include_timestamps,
                    replacement_rules,
                    replacement_prefix,
                    &transcript_dest,
                ) {
                    Ok(path) => Some(path),
                    Err(err) => {
                        queue[index].status = TranscribeItemStatus::Error;
                        queue[index].progress = 1.0;
                        queue[index].error = Some(err.to_string());
                        self.emit_queue_state(
                            TranscribeState::Transcribing,
                            queue.to_vec(),
                            Some(overall_progress(queue)),
                            Some(ProcessingStage::WritingTranscript),
                            None,
                        );
                        continue;
                    }
                }
            } else {
                None
            };

            // Persist the canonical record — always, regardless of the markdown toggle.
            let record = HistoryRecord::from_transcribe(
                input.display_name.clone(),
                model_name.to_string(),
                segments.clone(),
                replacement_rules,
                replacement_prefix,
                dual_source,
                input.source_path.to_string_lossy().into_owned(),
                markdown_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned()),
            );
            if let Err(e) = self.history.append(&save_folder, record) {
                tracing::warn!(error = %e, "failed to append transcribe history record");
            } else {
                self.app.emit("history://item-added", ()).ok();
            }

            queue[index].status = TranscribeItemStatus::Done;
            queue[index].progress = 1.0;
            queue[index].transcript_path = markdown_path.map(|p| p.to_string_lossy().to_string());
            queue[index].error = None;

            self.emit_queue_state(
                TranscribeState::Transcribing,
                queue.to_vec(),
                Some(overall_progress(queue)),
                Some(ProcessingStage::WritingTranscript),
                None,
            );
        }

        Ok(queue.to_vec())
    }

    fn emit_queue_state(
        &self,
        state: TranscribeState,
        items: Vec<TranscribeQueueItem>,
        progress: Option<f32>,
        processing_stage: Option<ProcessingStage>,
        error: Option<String>,
    ) {
        let mut event = TranscribeStateEvent::new(state, items);
        event.progress = progress;
        event.processing_stage = processing_stage;
        event.error = error;
        self.app.emit("transcribe://state-changed", event).ok();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn build_queue_items(inputs: &[TranscribeInputItem]) -> Vec<TranscribeQueueItem> {
    inputs
        .iter()
        .map(|input| TranscribeQueueItem {
            id: input.id.clone(),
            source_path: input.source_path.to_string_lossy().to_string(),
            display_name: input.display_name.clone(),
            source_type: input.source_type.clone(),
            duration_ms: input.duration_ms,
            status: TranscribeItemStatus::Queued,
            progress: 0.0,
            transcript_path: None,
            error: None,
        })
        .collect()
}

fn resolve_output_folder(
    output: &OutputService,
    config: &Config,
    output_folder: Option<&str>,
) -> Result<PathBuf, String> {
    let chosen = output_folder.unwrap_or(&config.save_folder).trim();
    if chosen.is_empty() {
        return Err("output folder cannot be empty".to_string());
    }
    let path = Path::new(chosen);
    if !path.is_absolute() {
        return Err(format!("output folder `{chosen}` must be absolute"));
    }
    output.ensure_output_dir(path).map_err(|e| e.to_string())
}

fn resolve_model_path(
    config: &Config,
    model: &ModelService,
    explicit_model_id: Option<&str>,
) -> PathBuf {
    if let Some(model_id) = explicit_model_id {
        if let Some(path) = model.model_path_for_id(model_id.trim()) {
            return path;
        }
    }
    if let Some(model_id) = &config.selected_model_id {
        if let Some(path) = model.model_path_for_id(model_id) {
            return path;
        }
    }
    model.default_model_path()
}

fn slugify(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    let mut slug: String = stem
        .chars()
        .map(|c| match c {
            ' ' => '_',
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect();
    if slug.is_empty() {
        slug = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    }
    slug
}

fn overall_progress(items: &[TranscribeQueueItem]) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    let total: f32 = items
        .iter()
        .map(|item| match item.status {
            TranscribeItemStatus::Done | TranscribeItemStatus::Error => 1.0,
            TranscribeItemStatus::Processing => item.progress.clamp(0.0, 1.0),
            TranscribeItemStatus::Queued => 0.0,
        })
        .sum();
    (total / items.len() as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Config, TranscribeItemStatus, TranscribeQueueItem};

    fn make_queue_item(status: TranscribeItemStatus, progress: f32) -> TranscribeQueueItem {
        TranscribeQueueItem {
            id: "test".to_string(),
            source_path: "".to_string(),
            display_name: "".to_string(),
            source_type: crate::types::TranscribeSourceType::SingleAudio,
            duration_ms: 0,
            status,
            progress,
            transcript_path: None,
            error: None,
        }
    }

    #[test]
    fn overall_progress_empty_returns_zero() {
        assert_eq!(overall_progress(&[]), 0.0);
    }

    #[test]
    fn overall_progress_all_done() {
        let items = vec![
            make_queue_item(TranscribeItemStatus::Done, 1.0),
            make_queue_item(TranscribeItemStatus::Done, 1.0),
        ];
        assert_eq!(overall_progress(&items), 1.0);
    }

    #[test]
    fn overall_progress_all_queued() {
        let items = vec![
            make_queue_item(TranscribeItemStatus::Queued, 0.0),
            make_queue_item(TranscribeItemStatus::Queued, 0.0),
        ];
        assert_eq!(overall_progress(&items), 0.0);
    }

    #[test]
    fn overall_progress_mixed() {
        let items = vec![
            make_queue_item(TranscribeItemStatus::Done, 1.0),
            make_queue_item(TranscribeItemStatus::Queued, 0.0),
            make_queue_item(TranscribeItemStatus::Processing, 0.5),
            make_queue_item(TranscribeItemStatus::Error, 1.0),
        ];
        // (1.0 + 0.0 + 0.5 + 1.0) / 4 = 0.625
        let p = overall_progress(&items);
        assert!((p - 0.625).abs() < 1e-5, "expected 0.625, got {p}");
    }

    #[test]
    fn slugify_spaces_become_underscores() {
        assert_eq!(slugify("hello world"), "hello_world");
    }

    #[test]
    fn slugify_strips_extension() {
        assert_eq!(slugify("audio file.wav"), "audio_file");
    }

    #[test]
    fn slugify_replaces_forbidden_chars() {
        let slug = slugify("a/b\\c:d");
        assert!(!slug.contains('/') && !slug.contains('\\') && !slug.contains(':'));
    }

    #[test]
    fn resolve_model_path_prefers_explicit_id() {
        let tmp = tempfile::tempdir().unwrap();
        let model_svc = ModelService::new(tmp.path().to_path_buf());
        let config = Config {
            selected_model_id: Some("base-en-q5".to_string()),
            ..Config::default()
        };
        // explicit id "tiny-en-q5" — path doesn't exist but model_path_for_id still returns it
        let path = resolve_model_path(&config, &model_svc, Some("tiny-en-q5"));
        assert!(path.to_string_lossy().contains("tiny"));
    }

    #[test]
    fn resolve_model_path_falls_back_to_selected() {
        let tmp = tempfile::tempdir().unwrap();
        let model_svc = ModelService::new(tmp.path().to_path_buf());
        let config = Config {
            selected_model_id: Some("base-en-q5".to_string()),
            ..Config::default()
        };
        let path = resolve_model_path(&config, &model_svc, None);
        assert!(path.to_string_lossy().contains("base"));
    }

    #[test]
    fn resolve_output_folder_rejects_relative_path() {
        let tmp = tempfile::tempdir().unwrap();
        let output_svc = crate::services::output::OutputService::new();
        let config = Config {
            save_folder: tmp.path().to_string_lossy().to_string(),
            ..Config::default()
        };
        let result = resolve_output_folder(&output_svc, &config, Some("relative/path"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be absolute"));
    }

    #[test]
    fn resolve_output_folder_rejects_empty() {
        let output_svc = crate::services::output::OutputService::new();
        let config = Config {
            save_folder: String::new(),
            ..Config::default()
        };
        let result = resolve_output_folder(&output_svc, &config, None);
        assert!(result.is_err());
    }
}
