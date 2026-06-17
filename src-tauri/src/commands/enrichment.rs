use crate::services::{enrichment::EnrichmentQueue, history::HistoryService, layers::LayerRegistry};
use crate::types::{
    AppError, AssignedItem, EnrichmentConfig, EnrichmentFlow, EnrichmentLayer, EnrichmentStep,
    FlowResult, ResultStatus,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_enrichment_config(
    registry: State<Arc<LayerRegistry>>,
) -> Result<EnrichmentConfig, AppError> {
    Ok(registry.get_config())
}

#[tauri::command]
pub fn upsert_layer(
    layer: EnrichmentLayer,
    registry: State<Arc<LayerRegistry>>,
) -> Result<(), AppError> {
    registry.upsert_layer(layer).map_err(AppError::from)
}

#[tauri::command]
pub fn upsert_step(
    step: EnrichmentStep,
    registry: State<Arc<LayerRegistry>>,
) -> Result<(), AppError> {
    registry.upsert_step(step).map_err(AppError::from)
}

#[tauri::command]
pub fn upsert_flow(
    flow: EnrichmentFlow,
    registry: State<Arc<LayerRegistry>>,
) -> Result<(), AppError> {
    registry.upsert_flow(flow).map_err(AppError::from)
}

#[tauri::command]
pub fn enqueue_flow_run(
    save_folder: String,
    record_id: String,
    flow_id: String,
    queue: State<Arc<EnrichmentQueue>>,
) -> Result<(), AppError> {
    queue.enqueue(save_folder, record_id, flow_id);
    Ok(())
}

/// Update the enrichment result for a layer on a transcript (edit or approve).
/// On Approve, new item names are promoted into the layer's shared vocabulary.
#[tauri::command]
pub fn update_flow_result(
    save_folder: String,
    record_id: String,
    layer_id: String,
    items: Vec<AssignedItem>,
    status: ResultStatus,
    registry: State<Arc<LayerRegistry>>,
    history: State<Arc<HistoryService>>,
) -> Result<(), AppError> {
    let result = FlowResult {
        layer_id: layer_id.clone(),
        items: items.clone(),
        status: status.clone(),
    };

    let mut results = std::collections::HashMap::new();
    results.insert(layer_id.clone(), result);
    history
        .update_enrichment(&save_folder, &record_id, results)
        .map_err(AppError::from)?;

    if status == ResultStatus::Approved {
        registry
            .promote_items(&layer_id, &items)
            .map_err(AppError::from)?;
    }

    Ok(())
}
