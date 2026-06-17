use crate::types::{ChunkStrategy, Segment};

/// Approximate token count for a segment's text. 1 token ≈ 4 chars is a safe undercount
/// for English prose; using it keeps chunks below context limits without complex tokenization.
fn approx_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Split segments into chunks according to the step's strategy.
///
/// `Chunk` splits on natural Segment boundaries, keeping each chunk under `max_tokens`.
/// `Full` returns all segments as a single chunk regardless of size.
pub fn chunk(segments: &[Segment], strategy: &ChunkStrategy, max_tokens: usize) -> Vec<Vec<Segment>> {
    match strategy {
        ChunkStrategy::Full => vec![segments.to_vec()],
        ChunkStrategy::Chunk => chunk_by_tokens(segments, max_tokens),
    }
}

fn chunk_by_tokens(segments: &[Segment], max_tokens: usize) -> Vec<Vec<Segment>> {
    if segments.is_empty() {
        return vec![];
    }

    let mut chunks: Vec<Vec<Segment>> = Vec::new();
    let mut current: Vec<Segment> = Vec::new();
    let mut current_tokens: usize = 0;

    for seg in segments {
        let seg_tokens = approx_tokens(&seg.text);
        // Always include at least one segment per chunk even if it exceeds the limit.
        if !current.is_empty() && current_tokens + seg_tokens > max_tokens {
            chunks.push(std::mem::take(&mut current));
            current_tokens = 0;
        }
        current_tokens += seg_tokens;
        current.push(seg.clone());
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// Render segments to a plain transcript string for passing to the LLM.
pub fn segments_to_text(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|s| s.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str) -> Segment {
        Segment { start_ms: 0, end_ms: 1000, text: text.to_string() }
    }

    #[test]
    fn full_strategy_returns_one_chunk() {
        let segs = vec![seg("hello world"), seg("foo bar baz")];
        let chunks = chunk(&segs, &ChunkStrategy::Full, 10);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 2);
    }

    #[test]
    fn chunk_strategy_splits_on_token_boundary() {
        // Each segment is ~25 chars → ~6 tokens. max_tokens=8 fits one segment per chunk.
        let segs: Vec<_> = (0..4).map(|i| seg(&format!("word word word word word {i}"))).collect();
        let chunks = chunk(&segs, &ChunkStrategy::Chunk, 8);
        assert!(chunks.len() > 1);
    }

    #[test]
    fn single_oversized_segment_gets_its_own_chunk() {
        let long = "a".repeat(200);
        let segs = vec![seg(&long), seg("short")];
        let chunks = chunk(&segs, &ChunkStrategy::Chunk, 10);
        // Long segment must not be dropped even though it exceeds max_tokens.
        assert_eq!(chunks[0][0].text, long);
    }

    #[test]
    fn segments_to_text_joins_trimmed() {
        let segs = vec![seg("  hello  "), seg("world"), seg("")];
        assert_eq!(segments_to_text(&segs), "hello world");
    }
}
