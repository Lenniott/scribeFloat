use anyhow::{anyhow, Context, Result};
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, LlamaModel, Special},
    sampling::LlamaSampler,
    token::data_array::LlamaTokenDataArray,
};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

pub const LLM_MODEL_FILENAME: &str = "gemma-4-E2B-it-qat-q4_0.gguf";

/// System prompt applied to every extraction call.
const SYSTEM_PROMPT: &str = "You are a precise data extraction assistant. \
    Extract only what is explicitly mentioned or clearly implied in the transcript. \
    Never invent content. Return valid JSON matching the requested schema exactly.";

/// Maximum tokens to generate per call. Extraction tasks produce short JSON arrays.
const MAX_NEW_TOKENS: i32 = 512;

/// Context length — enough for a full transcript chunk plus the prompt.
const CTX_SIZE: u32 = 8192;

static BACKEND: OnceLock<Arc<LlamaBackend>> = OnceLock::new();

fn get_backend() -> Result<Arc<LlamaBackend>> {
    BACKEND
        .get_or_init(|| {
            Arc::new(LlamaBackend::init().expect("llama backend init"))
        })
        .clone()
        .pipe_ok()
}

trait PipeOk<T> {
    fn pipe_ok(self) -> Result<T>;
}
impl<T: Clone> PipeOk<T> for T {
    fn pipe_ok(self) -> Result<T> {
        Ok(self)
    }
}

pub struct LocalLLMService {
    model_path: PathBuf,
    /// Loaded model, kept warm after first use.
    loaded_model: Mutex<Option<Arc<LlamaModel>>>,
    /// Serializes all llama.cpp inference calls. Separate from ModelService::inference_gate
    /// in spike v1 — v2 should unify into a single app-wide ggml gate.
    inference_gate: Mutex<()>,
}

impl LocalLLMService {
    pub fn new(models_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            model_path: models_dir.join(LLM_MODEL_FILENAME),
            loaded_model: Mutex::new(None),
            inference_gate: Mutex::new(()),
        })
    }

    pub fn model_path(&self) -> &PathBuf {
        &self.model_path
    }

    pub fn model_available(&self) -> bool {
        self.model_path.exists()
            && std::fs::metadata(&self.model_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false)
    }

    fn get_or_load_model(&self) -> Result<Arc<LlamaModel>> {
        let mut guard = self.loaded_model.lock().unwrap();
        if let Some(m) = guard.as_ref() {
            return Ok(m.clone());
        }
        if !self.model_available() {
            return Err(anyhow!(
                "Gemma model not found at {:?}. Download it from HuggingFace first.",
                self.model_path
            ));
        }
        let _backend = get_backend()?;
        let params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&self.model_path, params)
            .with_context(|| format!("load Gemma model from {:?}", self.model_path))?;
        let arc = Arc::new(model);
        *guard = Some(arc.clone());
        Ok(arc)
    }

    /// Run a single bounded extraction call.
    ///
    /// `user_content` is the transcript text + task instruction.
    /// `json_schema` is a JSON Schema object string; grammar-constrained decoding ensures
    /// the model returns valid JSON matching it exactly.
    pub fn extract(&self, user_content: &str, json_schema: &str) -> Result<String> {
        let _gate = self.inference_gate.lock().unwrap();
        let model = self.get_or_load_model()?;

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(CTX_SIZE));
        let mut ctx = model
            .new_context(&llama_cpp_2::llama_backend::LlamaBackend::init()?, ctx_params)
            .context("create llama context")?;

        // Format as a chat prompt using Gemma's instruction template.
        let prompt = format!(
            "<start_of_turn>system\n{SYSTEM_PROMPT}<end_of_turn>\n\
             <start_of_turn>user\n{user_content}<end_of_turn>\n\
             <start_of_turn>model\n"
        );

        let tokens = model
            .str_to_token(&prompt, Special::Tokenize)
            .context("tokenize prompt")?;

        let n_prompt = tokens.len() as i32;
        let mut batch = LlamaBatch::new(CTX_SIZE as usize, 1);
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch.add(token, i as i32, &[0], is_last)?;
        }
        ctx.decode(&mut batch).context("decode prompt")?;

        // Grammar-constrained sampler: temperature + grammar forces valid JSON output.
        let grammar_sampler = LlamaSampler::grammar(&model, json_schema, "root")
            .context("build grammar sampler")?;
        let mut sampler = LlamaSampler::chain(
            [
                LlamaSampler::temp(0.0), // greedy — extraction doesn't need creativity
                grammar_sampler,
                LlamaSampler::greedy(),
            ],
            false,
        );

        let mut output_tokens: Vec<llama_cpp_2::token::LlamaToken> = Vec::new();
        let mut n_pos = n_prompt;

        loop {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }
            if output_tokens.len() as i32 >= MAX_NEW_TOKENS {
                break;
            }

            output_tokens.push(token);

            batch.clear();
            batch.add(token, n_pos, &[0], true)?;
            ctx.decode(&mut batch).context("decode token")?;
            n_pos += 1;
        }

        let text = output_tokens
            .iter()
            .map(|&t| model.token_to_str(t, Special::Tokenize).unwrap_or_default())
            .collect::<String>();

        Ok(text)
    }
}
