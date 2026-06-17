use crate::services::{
    chunker::{chunk, segments_to_text},
    history::HistoryService,
    layers::LayerRegistry,
    llm::LocalLLMService,
};
use crate::types::{
    AssignedItem, ChunkStrategy, EnrichmentJobStatus, EnrichmentStatusEvent, FlowResult,
    ResultStatus,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

const MAX_CHUNK_TOKENS: usize = 4096;
const ENRICHMENT_STATUS_EVENT: &str = "enrichment-status";

#[derive(Debug)]
pub struct EnrichmentJob {
    pub save_folder: String,
    pub record_id: String,
    pub flow_id: String,
}

pub struct EnrichmentQueue {
    tx: mpsc::UnboundedSender<EnrichmentJob>,
}

impl EnrichmentQueue {
    pub fn new(
        app: AppHandle,
        llm: Arc<LocalLLMService>,
        layers: Arc<LayerRegistry>,
        history: Arc<HistoryService>,
    ) -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(run_worker(rx, app, llm, layers, history));
        Arc::new(Self { tx })
    }

    pub fn enqueue(&self, save_folder: String, record_id: String, flow_id: String) {
        let _ = self.tx.send(EnrichmentJob { save_folder, record_id, flow_id });
    }
}

async fn run_worker(
    mut rx: mpsc::UnboundedReceiver<EnrichmentJob>,
    app: AppHandle,
    llm: Arc<LocalLLMService>,
    layers: Arc<LayerRegistry>,
    history: Arc<HistoryService>,
) {
    while let Some(job) = rx.recv().await {
        emit_status(&app, &job.record_id, &job.flow_id, EnrichmentJobStatus::Running, None);

        let result = tokio::task::spawn_blocking({
            let llm = llm.clone();
            let layers = layers.clone();
            let history = history.clone();
            let save_folder = job.save_folder.clone();
            let record_id = job.record_id.clone();
            let flow_id = job.flow_id.clone();
            move || run_flow(&save_folder, &record_id, &flow_id, &llm, &layers, &history)
        })
        .await;

        match result {
            Ok(Ok(())) => {
                emit_status(&app, &job.record_id, &job.flow_id, EnrichmentJobStatus::Done, None);
            }
            Ok(Err(e)) => {
                tracing::error!(record_id = %job.record_id, flow_id = %job.flow_id, error = %e, "enrichment failed");
                emit_status(&app, &job.record_id, &job.flow_id, EnrichmentJobStatus::Error, Some(e.to_string()));
            }
            Err(e) => {
                tracing::error!(record_id = %job.record_id, flow_id = %job.flow_id, error = %e, "enrichment task panicked");
                emit_status(&app, &job.record_id, &job.flow_id, EnrichmentJobStatus::Error, Some("internal error".to_string()));
            }
        }
    }
}

fn run_flow(
    save_folder: &str,
    record_id: &str,
    flow_id: &str,
    llm: &LocalLLMService,
    layers: &LayerRegistry,
    history: &HistoryService,
) -> Result<()> {
    let flow = layers
        .get_flow(flow_id)
        .with_context(|| format!("flow {flow_id} not found"))?;

    let record = history
        .get(save_folder, record_id)
        .with_context(|| format!("load record {record_id}"))?
        .with_context(|| format!("record {record_id} not found"))?;

    let mut results: std::collections::HashMap<String, FlowResult> = std::collections::HashMap::new();

    for step_id in &flow.steps {
        let step = match layers.get_step(step_id) {
            Some(s) => s,
            None => {
                tracing::warn!(step_id, "step not found — skipping");
                continue;
            }
        };
        let layer = match layers.get_layer(&step.layer_id) {
            Some(l) => l,
            None => {
                tracing::warn!(layer_id = %step.layer_id, "layer not found — skipping step");
                continue;
            }
        };

        let chunks = chunk(&record.segments, &step.chunk_strategy, MAX_CHUNK_TOKENS);
        let mut step_items: Vec<AssignedItem> = Vec::new();

        for chunk_segs in &chunks {
            let transcript_text = segments_to_text(chunk_segs);
            if transcript_text.is_empty() {
                continue;
            }

            let vocab_hint = if layer.unique_list && !layer.vocabulary.is_empty() {
                let names: Vec<&str> = layer.vocabulary.iter().map(|i| i.name.as_str()).collect();
                format!("\n\nExisting vocabulary (reuse these terms where appropriate): {}", names.join(", "))
            } else {
                String::new()
            };

            let user_content = format!(
                "{}\n\nTranscript:\n{transcript_text}{vocab_hint}",
                step.prompt
            );

            let schema = build_json_schema(layer.per_item_description);
            let raw = llm.extract(&user_content, &schema)
                .with_context(|| format!("LLM call for step {step_id}"))?;

            let mut parsed = parse_items(&raw, layer.per_item_description)
                .with_context(|| format!("parse LLM output for step {step_id}: {raw:?}"))?;
            step_items.append(&mut parsed);
        }

        // Deduplicate by name (case-insensitive) when unique_list is on.
        if layer.unique_list {
            let mut seen = std::collections::HashSet::new();
            step_items.retain(|item| seen.insert(item.name.to_lowercase()));
        }

        results.insert(
            step.layer_id.clone(),
            FlowResult {
                layer_id: step.layer_id.clone(),
                items: step_items,
                status: ResultStatus::Draft,
            },
        );
    }

    history.update_enrichment(save_folder, record_id, results)
        .context("write enrichment results to history")?;

    Ok(())
}

/// JSON Schema string for the LLM output, varying by whether items have descriptions.
fn build_json_schema(per_item_description: bool) -> String {
    if per_item_description {
        r#"{
  "type": "object",
  "properties": {
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "name": {"type": "string"},
          "description": {"type": "string"}
        },
        "required": ["name", "description"],
        "additionalProperties": false
      }
    }
  },
  "required": ["items"],
  "additionalProperties": false
}"#
        .to_string()
    } else {
        r#"{
  "type": "object",
  "properties": {
    "items": {
      "type": "array",
      "items": {"type": "string"}
    }
  },
  "required": ["items"],
  "additionalProperties": false
}"#
        .to_string()
    }
}

fn parse_items(raw: &str, per_item_description: bool) -> Result<Vec<AssignedItem>> {
    let v: serde_json::Value =
        serde_json::from_str(raw.trim()).context("JSON parse of LLM output")?;

    let items_arr = v
        .get("items")
        .and_then(|a| a.as_array())
        .with_context(|| "LLM output missing 'items' array")?;

    if per_item_description {
        items_arr
            .iter()
            .map(|item| {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .with_context(|| "item missing 'name'")?
                    .trim()
                    .to_string();
                let description = item
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                Ok(AssignedItem { name, description })
            })
            .collect()
    } else {
        Ok(items_arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| AssignedItem { name: s.trim().to_string(), description: None })
            .filter(|i| !i.name.is_empty())
            .collect())
    }
}

fn emit_status(
    app: &AppHandle,
    record_id: &str,
    flow_id: &str,
    status: EnrichmentJobStatus,
    error: Option<String>,
) {
    let _ = app.emit(
        ENRICHMENT_STATUS_EVENT,
        EnrichmentStatusEvent {
            record_id: record_id.to_string(),
            flow_id: flow_id.to_string(),
            status,
            error,
        },
    );
}
