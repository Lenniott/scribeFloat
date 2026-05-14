use crate::services::config::ConfigService;
use crate::services::model::ModelService;
use crate::services::output::OutputService;
use crate::services::transcribe_input::{TranscribeInputItem, TranscribeInputService};
use crate::types::{
    Config, ProcessingStage, TranscribeItemStatus, TranscribeQueueItem, TranscribeState,
    TranscribeStateEvent,
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
    config: Arc<ConfigService>,
    app: AppHandle,
}

impl TranscribeController {
    pub fn new(
        input: Arc<TranscribeInputService>,
        model: Arc<ModelService>,
        output: Arc<OutputService>,
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
            config,
            app,
        })
    }

    pub fn inspect_inputs(&self, input_paths: Vec<String>) -> Result<Vec<TranscribeQueueItem>, String> {
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
        tauri::async_runtime::spawn(async move {
            let ctrl = Arc::clone(&this);
            let result = tokio::task::spawn_blocking(move || {
                ctrl.run_batch(inputs, &model_path, &model_name, &output_folder, include_timestamps, &replacement_rules, &mut queue)
            })
            .await;

            match result {
                Ok(Ok(final_queue)) => {
                    this.lock().state = TranscribeState::Done;
                    this.emit_queue_state(TranscribeState::Done, final_queue, Some(1.0), None, None);
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
        queue: &mut [TranscribeQueueItem],
    ) -> Result<Vec<TranscribeQueueItem>, String> {
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
            let vad = self.model.model_available(&vad_path).then_some(vad_path.as_path());
            let segments = if let Some(speaker_pcm) = decoded.speaker_pcm_16k.as_ref() {
                let mic_segments = self
                    .model
                    .transcribe_pcm_with_progress(model_path, &decoded.mic_pcm_16k, vad, {
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
                    })
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
                    .transcribe_pcm_with_progress(model_path, speaker_pcm, vad, {
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
                    })
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

                self.model.merge_dual_source(&mic_segments, &speaker_segments)
            } else {
                match self.model.transcribe_pcm_with_progress(
                    model_path,
                    &decoded.mic_pcm_16k,
                    vad,
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

            let output_name = format!("{}_{}.md", slugify(&input.display_name), model_name);
            let transcript_dest = output_folder.join(output_name);
            match self.output.write_transcript(
                &segments,
                &[],
                &input.display_name,
                model_name,
                include_timestamps,
                replacement_rules,
                &transcript_dest,
            ) {
                Ok(path) => {
                    queue[index].status = TranscribeItemStatus::Done;
                    queue[index].progress = 1.0;
                    queue[index].transcript_path = Some(path.to_string_lossy().to_string());
                    queue[index].error = None;
                }
                Err(err) => {
                    queue[index].status = TranscribeItemStatus::Error;
                    queue[index].progress = 1.0;
                    queue[index].error = Some(err.to_string());
                }
            }

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

fn resolve_model_path(config: &Config, model: &ModelService, explicit_model_id: Option<&str>) -> PathBuf {
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
