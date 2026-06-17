use crate::types::{
    AssignedItem, ChunkStrategy, EnrichmentConfig, EnrichmentFlow, EnrichmentLayer, EnrichmentStep,
    FlowTrigger, RenderType,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct LayerRegistry {
    config_path: PathBuf,
    state: Mutex<EnrichmentConfig>,
}

impl LayerRegistry {
    pub fn load_or_seed(config_path: PathBuf) -> Result<std::sync::Arc<Self>> {
        let config = if config_path.exists() {
            let raw = std::fs::read_to_string(&config_path)
                .with_context(|| format!("read enrichment config {config_path:?}"))?;
            serde_json::from_str::<EnrichmentConfig>(&raw)
                .with_context(|| "deserialize enrichment config")?
        } else {
            let seeded = seed_defaults();
            let raw = serde_json::to_string_pretty(&seeded)?;
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&config_path, raw)?;
            seeded
        };

        Ok(std::sync::Arc::new(Self {
            config_path,
            state: Mutex::new(config),
        }))
    }

    pub fn get_config(&self) -> EnrichmentConfig {
        self.state.lock().unwrap().clone()
    }

    pub fn get_on_creation_flow(&self) -> Option<EnrichmentFlow> {
        let state = self.state.lock().unwrap();
        state
            .flows
            .iter()
            .find(|f| f.trigger == FlowTrigger::OnCreation)
            .cloned()
    }

    pub fn get_flow(&self, flow_id: &str) -> Option<EnrichmentFlow> {
        let state = self.state.lock().unwrap();
        state.flows.iter().find(|f| f.id == flow_id).cloned()
    }

    pub fn get_step(&self, step_id: &str) -> Option<EnrichmentStep> {
        let state = self.state.lock().unwrap();
        state.steps.iter().find(|s| s.id == step_id).cloned()
    }

    pub fn get_layer(&self, layer_id: &str) -> Option<EnrichmentLayer> {
        let state = self.state.lock().unwrap();
        state.layers.iter().find(|l| l.id == layer_id).cloned()
    }

    pub fn upsert_layer(&self, layer: EnrichmentLayer) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.layers.iter_mut().find(|l| l.id == layer.id) {
            *existing = layer;
        } else {
            state.layers.push(layer);
        }
        self.save_locked(&state)
    }

    pub fn upsert_step(&self, step: EnrichmentStep) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.steps.iter_mut().find(|s| s.id == step.id) {
            *existing = step;
        } else {
            state.steps.push(step);
        }
        self.save_locked(&state)
    }

    pub fn upsert_flow(&self, flow: EnrichmentFlow) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        // If this flow claims on-creation, clear the previous holder.
        if flow.trigger == FlowTrigger::OnCreation {
            for existing in state.flows.iter_mut() {
                if existing.id != flow.id && existing.trigger == FlowTrigger::OnCreation {
                    existing.trigger = FlowTrigger::Manual;
                }
            }
        }
        if let Some(existing) = state.flows.iter_mut().find(|f| f.id == flow.id) {
            *existing = flow;
        } else {
            state.flows.push(flow);
        }
        self.save_locked(&state)
    }

    /// Promote new item names into the layer's vocabulary. Called on Approve.
    /// Items already in the vocabulary (matched by name, case-insensitive) are not duplicated.
    pub fn promote_items(&self, layer_id: &str, new_items: &[AssignedItem]) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if let Some(layer) = state.layers.iter_mut().find(|l| l.id == layer_id) {
            let existing_names: std::collections::HashSet<String> = layer
                .vocabulary
                .iter()
                .map(|i| i.name.to_lowercase())
                .collect();
            for item in new_items {
                if !existing_names.contains(&item.name.to_lowercase()) {
                    layer.vocabulary.push(item.clone());
                }
            }
        }
        self.save_locked(&state)
    }

    fn save_locked(&self, state: &EnrichmentConfig) -> Result<()> {
        let raw = serde_json::to_string_pretty(state)?;
        let tmp = self.config_path.with_extension("tmp");
        std::fs::write(&tmp, raw)?;
        std::fs::rename(&tmp, &self.config_path)?;
        Ok(())
    }
}

fn seed_defaults() -> EnrichmentConfig {
    let tags_layer = EnrichmentLayer {
        id: "layer_tags".to_string(),
        name: "Tags".to_string(),
        description: Some("Project areas, themes, and topics mentioned in this transcript.".to_string()),
        unique_list: true,
        per_item_description: false,
        render_type: RenderType::ChipList,
        vocabulary: vec![],
    };

    let keywords_layer = EnrichmentLayer {
        id: "layer_keywords".to_string(),
        name: "Keywords".to_string(),
        description: Some("Key terms, concepts, and named entities from this transcript.".to_string()),
        unique_list: false,
        per_item_description: false,
        render_type: RenderType::ChipList,
        vocabulary: vec![],
    };

    let tags_step = EnrichmentStep {
        id: "step_tags".to_string(),
        name: "Extract Tags".to_string(),
        layer_id: "layer_tags".to_string(),
        prompt: "Extract project areas, themes, and topics from this transcript. \
            Return only tags that are genuinely meaningful for design work. \
            Prefer existing vocabulary terms where they fit. \
            Avoid generic words like 'design', 'project', 'meeting'.".to_string(),
        chunk_strategy: ChunkStrategy::Chunk,
    };

    let keywords_step = EnrichmentStep {
        id: "step_keywords".to_string(),
        name: "Extract Keywords".to_string(),
        layer_id: "layer_keywords".to_string(),
        prompt: "Extract the most important specific terms, named features, components, \
            and concepts from this transcript. Focus on concrete nouns the designer \
            would search for later.".to_string(),
        chunk_strategy: ChunkStrategy::Full,
    };

    let default_flow = EnrichmentFlow {
        id: "flow_default".to_string(),
        name: "Tags & Keywords".to_string(),
        steps: vec!["step_tags".to_string(), "step_keywords".to_string()],
        trigger: FlowTrigger::OnCreation,
    };

    EnrichmentConfig {
        layers: vec![tags_layer, keywords_layer],
        steps: vec![tags_step, keywords_step],
        flows: vec![default_flow],
    }
}
