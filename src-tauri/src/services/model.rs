use crate::types::Segment;
use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct ModelService;

impl ModelService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn model_available(&self, path: &Path) -> bool {
        path.exists()
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
                    start_ms: t0 * 10, // centiseconds → ms
                    end_ms: t1 * 10,
                    text,
                });
            }
        }

        Ok(segments)
    }
}
