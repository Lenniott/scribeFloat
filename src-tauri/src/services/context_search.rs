use crate::services::history::HistoryService;
use crate::services::note_sidecar;
use crate::types::{HistoryKind, HistoryRecord, Segment};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub const INDEX_SCHEMA_VERSION: u8 = 1;
pub const DEFAULT_MODEL_ID: &str = "Qdrant/bge-small-en-v1.5-onnx-Q";
pub const BASE_MODEL_ID: &str = "BAAI/bge-small-en-v1.5";
pub const BGE_MODEL_FILE: &str = "model_optimized.onnx";
pub const BGE_MODEL_SHA256: &str =
    "51f1bd0addd6e859e42c2c8021a5e5461385bb676a649f4b269aa445449f2431";
pub const BGE_QUERY_INSTRUCTION: &str = "Represent this sentence for searching relevant passages: ";

const CHUNK_TARGET_WORDS: usize = 420;
const CHUNK_MAX_WORDS: usize = 620;
const SNIPPET_MAX_CHARS: usize = 280;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextIndexManifest {
    pub schema_version: u8,
    pub model_id: String,
    pub embedding_dim: usize,
    pub generated_at: String,
    pub chunk_count: usize,
    pub corpus_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextChunk {
    pub id: String,
    pub note_id: String,
    pub note_title: String,
    pub note_kind: HistoryKind,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<i64>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScoreBreakdown {
    pub vector: f32,
    pub keyword: f32,
    pub tag: f32,
    pub recency: f32,
    pub total: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextSearchResult {
    pub chunk_id: String,
    pub note_id: String,
    pub note_title: String,
    pub note_kind: HistoryKind,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<i64>,
    pub snippet: String,
    pub matched_terms: Vec<String>,
    pub score: ScoreBreakdown,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub limit: usize,
    pub since_days: Option<i64>,
    pub tag: Option<String>,
}

impl SearchOptions {
    pub fn new(query: String) -> Self {
        Self {
            query,
            limit: 20,
            since_days: None,
            tag: None,
        }
    }
}

pub trait EmbeddingProvider {
    fn model_id(&self) -> &'static str;
    fn embed_documents(&mut self, documents: &[String]) -> Result<Vec<Vec<f32>>>;
    fn embed_query(&mut self, query: &str) -> Result<Vec<f32>>;
}

pub struct FastEmbedProvider {
    model: fastembed::TextEmbedding,
}

impl FastEmbedProvider {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let mut options =
            fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGESmallENV15Q);
        options.cache_dir = cache_dir;
        options.show_download_progress = true;
        let model =
            fastembed::TextEmbedding::try_new(options).context("load BGE embedding model")?;
        Ok(Self { model })
    }

    pub fn new_verified(cache_dir: PathBuf) -> Result<Self> {
        let provider = Self::new(cache_dir.clone())?;
        verify_bge_model_cache(&cache_dir)?;
        Ok(provider)
    }
}

impl EmbeddingProvider for FastEmbedProvider {
    fn model_id(&self) -> &'static str {
        DEFAULT_MODEL_ID
    }

    fn embed_documents(&mut self, documents: &[String]) -> Result<Vec<Vec<f32>>> {
        let embeddings = self
            .model
            .embed(documents, None)
            .context("embed document chunks")?;
        Ok(embeddings.into_iter().map(normalize).collect())
    }

    fn embed_query(&mut self, query: &str) -> Result<Vec<f32>> {
        let text = format!("{BGE_QUERY_INSTRUCTION}{query}");
        let mut embeddings = self
            .model
            .embed(vec![text], None)
            .context("embed search query")?;
        let embedding = embeddings
            .pop()
            .ok_or_else(|| anyhow!("embedding provider returned no query vector"))?;
        Ok(normalize(embedding))
    }
}

pub fn index_dir(save_folder: &str) -> PathBuf {
    PathBuf::from(save_folder).join(".indexes").join("context")
}

pub fn default_model_cache_dir(save_folder: &str) -> PathBuf {
    index_dir(save_folder).join("models")
}

pub fn verify_bge_model_cache(cache_dir: &Path) -> Result<()> {
    let candidates = find_files_named(cache_dir, BGE_MODEL_FILE)?;
    if candidates.is_empty() {
        return Err(anyhow!(
            "BGE model file `{}` was not found under {}",
            BGE_MODEL_FILE,
            cache_dir.display()
        ));
    }
    let mut mismatches = Vec::new();
    for candidate in candidates {
        let actual = file_sha256_hex(&candidate)
            .with_context(|| format!("hash BGE model {}", candidate.display()))?;
        if actual == BGE_MODEL_SHA256 {
            return Ok(());
        }
        mismatches.push(format!("{} => {}", candidate.display(), actual));
    }
    Err(anyhow!(
        "BGE model SHA-256 mismatch; expected {}, checked {}",
        BGE_MODEL_SHA256,
        mismatches.join(", ")
    ))
}

pub fn build_index(
    history: &HistoryService,
    save_folder: &str,
    provider: &mut dyn EmbeddingProvider,
) -> Result<ContextIndexManifest> {
    let records = history.list(save_folder)?;
    let chunks = chunk_records(save_folder, &records);
    let documents: Vec<String> = chunks.iter().map(|chunk| chunk.text.clone()).collect();
    let vectors = if documents.is_empty() {
        Vec::new()
    } else {
        provider.embed_documents(&documents)?
    };
    if vectors.len() != chunks.len() {
        return Err(anyhow!(
            "embedding count {} did not match chunk count {}",
            vectors.len(),
            chunks.len()
        ));
    }
    let embedding_dim = vectors.first().map(|v| v.len()).unwrap_or(0);
    let manifest = ContextIndexManifest {
        schema_version: INDEX_SCHEMA_VERSION,
        model_id: provider.model_id().to_string(),
        embedding_dim,
        generated_at: chrono::Utc::now().to_rfc3339(),
        chunk_count: chunks.len(),
        corpus_fingerprint: corpus_fingerprint(&records),
    };
    write_index(save_folder, &manifest, &chunks, &vectors)?;
    Ok(manifest)
}

pub fn search_index(
    save_folder: &str,
    provider: &mut dyn EmbeddingProvider,
    options: &SearchOptions,
) -> Result<Vec<ContextSearchResult>> {
    let (manifest, chunks, vectors) = read_index(save_folder)?;
    if manifest.schema_version != INDEX_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported context index schema {}; rebuild the index",
            manifest.schema_version
        ));
    }
    if manifest.model_id != provider.model_id() {
        return Err(anyhow!(
            "context index was built with {}; rebuild for {}",
            manifest.model_id,
            provider.model_id()
        ));
    }
    if chunks.len() != vectors.len() {
        return Err(anyhow!("context index chunks/vectors are out of sync"));
    }

    let query_vector = provider.embed_query(&options.query)?;
    let query_terms = extract_terms(&options.query);
    let now = chrono::Utc::now();
    let since = options
        .since_days
        .map(|days| now - chrono::Duration::days(days.max(0)));
    let tag_filter = options.tag.as_ref().map(|tag| normalize_tag(tag));

    let mut results: Vec<ContextSearchResult> = chunks
        .into_iter()
        .zip(vectors)
        .filter(|(chunk, _)| {
            if let Some(since) = since {
                let Ok(created) = chrono::DateTime::parse_from_rfc3339(&chunk.created_at) else {
                    return false;
                };
                if created.with_timezone(&chrono::Utc) < since {
                    return false;
                }
            }
            if let Some(tag) = tag_filter.as_ref() {
                return chunk
                    .tags
                    .iter()
                    .any(|candidate| normalize_tag(candidate) == *tag);
            }
            true
        })
        .map(|(chunk, vector)| score_chunk(chunk, vector, &query_vector, &query_terms, now))
        .filter(|result| result.score.total > 0.0)
        .collect();

    results.sort_by(|a, b| {
        b.score
            .total
            .partial_cmp(&a.score.total)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
    });
    results.truncate(options.limit.max(1));
    Ok(results)
}

pub fn export_context_pack(
    save_folder: &str,
    provider: &mut dyn EmbeddingProvider,
    options: &SearchOptions,
    out_path: &Path,
) -> Result<Vec<ContextSearchResult>> {
    let results = search_index(save_folder, provider, options)?;
    let markdown = render_context_pack(options, &results);
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("create context pack output directory")?;
        }
    }
    std::fs::write(out_path, markdown).context("write context pack")?;
    Ok(results)
}

pub fn render_context_pack(options: &SearchOptions, results: &[ContextSearchResult]) -> String {
    let mut out = String::new();
    out.push_str("# Context Pack\n\n");
    out.push_str(&format!("- Query: `{}`\n", options.query));
    out.push_str(&format!(
        "- Generated: `{}`\n",
        chrono::Utc::now().to_rfc3339()
    ));
    if let Some(days) = options.since_days {
        out.push_str(&format!("- Since: last {days} days\n"));
    }
    if let Some(tag) = options.tag.as_ref() {
        out.push_str(&format!("- Tag: `{tag}`\n"));
    }
    out.push('\n');

    if results.is_empty() {
        out.push_str("No matching chunks found.\n");
        return out;
    }

    for (idx, result) in results.iter().enumerate() {
        out.push_str(&format!(
            "## {}. {} ({})\n\n",
            idx + 1,
            result.note_title,
            result.created_at
        ));
        out.push_str(&format!("- Note ID: `{}`\n", result.note_id));
        out.push_str(&format!("- Chunk ID: `{}`\n", result.chunk_id));
        if let Some(anchor) = timestamp_anchor(result.start_ms, result.end_ms) {
            out.push_str(&format!("- Timestamp: `{anchor}`\n"));
        }
        if !result.matched_terms.is_empty() {
            out.push_str(&format!(
                "- Matched terms: {}\n",
                result
                    .matched_terms
                    .iter()
                    .map(|term| format!("`{term}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        out.push_str(&format!(
            "- Score: {:.3} (vector {:.3}, keyword {:.3}, tag {:.3}, recency {:.3})\n\n",
            result.score.total,
            result.score.vector,
            result.score.keyword,
            result.score.tag,
            result.score.recency
        ));
        out.push_str("> ");
        out.push_str(&result.snippet.replace('\n', "\n> "));
        out.push_str("\n\n");
    }
    out
}

pub fn chunk_records(save_folder: &str, records: &[HistoryRecord]) -> Vec<ContextChunk> {
    let mut chunks = Vec::new();
    for record in records.iter().filter(|record| !record.deleted) {
        let tags = note_sidecar::read_meta(save_folder, &record.id)
            .map(|meta| meta.tags)
            .unwrap_or_default();
        let mut note_chunks = match record.kind {
            HistoryKind::Written => chunk_written(record, &tags),
            HistoryKind::Dictate => chunk_dictate(record, &tags),
            HistoryKind::Scribe | HistoryKind::Transcribe => chunk_transcript(record, &tags),
        };
        chunks.append(&mut note_chunks);
    }
    chunks
}

fn chunk_transcript(record: &HistoryRecord, tags: &[String]) -> Vec<ContextChunk> {
    if !record.speaker_blocks.is_empty() {
        let mut chunks = Vec::new();
        let mut current = ChunkAccumulator::new(record, tags);
        for block in &record.speaker_blocks {
            let text = block.text.trim();
            if text.is_empty() {
                continue;
            }
            current.push(
                text,
                block.start_ms.map(|v| v as i64),
                block.end_ms.map(|v| v as i64),
            );
            if current.word_count >= CHUNK_TARGET_WORDS {
                chunks.push(current.finish(chunks.len()));
                current = ChunkAccumulator::new(record, tags);
            }
        }
        if !current.is_empty() {
            chunks.push(current.finish(chunks.len()));
        }
        return chunks;
    }
    chunk_segments(record, tags, &record.segments)
}

fn chunk_dictate(record: &HistoryRecord, tags: &[String]) -> Vec<ContextChunk> {
    let text = record
        .segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.split_whitespace().count() <= CHUNK_MAX_WORDS {
        return vec![make_chunk(
            record,
            tags,
            0,
            first_start_ms(&record.segments),
            last_end_ms(&record.segments),
            text,
        )]
        .into_iter()
        .filter(|chunk| !chunk.text.trim().is_empty())
        .collect();
    }
    chunk_plain_text(record, tags, &text)
}

fn chunk_written(record: &HistoryRecord, tags: &[String]) -> Vec<ContextChunk> {
    let content = record.written_content.as_deref().unwrap_or("").trim();
    if content.is_empty() {
        return Vec::new();
    }
    let blocks = markdown_blocks(content);
    chunk_text_blocks(record, tags, blocks)
}

fn chunk_segments(
    record: &HistoryRecord,
    tags: &[String],
    segments: &[Segment],
) -> Vec<ContextChunk> {
    let mut chunks = Vec::new();
    let mut current = ChunkAccumulator::new(record, tags);
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        current.push(text, Some(segment.start_ms), Some(segment.end_ms));
        if current.word_count >= CHUNK_TARGET_WORDS {
            chunks.push(current.finish(chunks.len()));
            current = ChunkAccumulator::new(record, tags);
        }
    }
    if !current.is_empty() {
        chunks.push(current.finish(chunks.len()));
    }
    chunks
}

fn chunk_plain_text(record: &HistoryRecord, tags: &[String], text: &str) -> Vec<ContextChunk> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();
    for group in words.chunks(CHUNK_TARGET_WORDS) {
        chunks.push(make_chunk(
            record,
            tags,
            chunks.len(),
            None,
            None,
            group.join(" "),
        ));
    }
    chunks
}

fn chunk_text_blocks(
    record: &HistoryRecord,
    tags: &[String],
    blocks: Vec<String>,
) -> Vec<ContextChunk> {
    let mut chunks = Vec::new();
    let mut buf = Vec::new();
    let mut words = 0usize;
    for block in blocks {
        let block_words = block.split_whitespace().count();
        if block.trim_start().starts_with('#') && !buf.is_empty() {
            chunks.push(make_chunk(
                record,
                tags,
                chunks.len(),
                None,
                None,
                buf.join("\n\n"),
            ));
            buf.clear();
            words = 0;
        }
        if !buf.is_empty() && words + block_words > CHUNK_MAX_WORDS {
            chunks.push(make_chunk(
                record,
                tags,
                chunks.len(),
                None,
                None,
                buf.join("\n\n"),
            ));
            buf.clear();
            words = 0;
        }
        buf.push(block);
        words += block_words;
        if words >= CHUNK_TARGET_WORDS {
            chunks.push(make_chunk(
                record,
                tags,
                chunks.len(),
                None,
                None,
                buf.join("\n\n"),
            ));
            buf.clear();
            words = 0;
        }
    }
    if !buf.is_empty() {
        chunks.push(make_chunk(
            record,
            tags,
            chunks.len(),
            None,
            None,
            buf.join("\n\n"),
        ));
    }
    chunks
}

fn markdown_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') && !current.is_empty() {
            blocks.push(current.join("\n"));
            current.clear();
        }
        if trimmed.is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line.to_string());
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

struct ChunkAccumulator<'a> {
    record: &'a HistoryRecord,
    tags: &'a [String],
    texts: Vec<String>,
    word_count: usize,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
}

impl<'a> ChunkAccumulator<'a> {
    fn new(record: &'a HistoryRecord, tags: &'a [String]) -> Self {
        Self {
            record,
            tags,
            texts: Vec::new(),
            word_count: 0,
            start_ms: None,
            end_ms: None,
        }
    }

    fn push(&mut self, text: &str, start_ms: Option<i64>, end_ms: Option<i64>) {
        self.word_count += text.split_whitespace().count();
        self.texts.push(text.to_string());
        self.start_ms = self.start_ms.or(start_ms);
        if end_ms.is_some() {
            self.end_ms = end_ms;
        }
    }

    fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    fn finish(self, ordinal: usize) -> ContextChunk {
        make_chunk(
            self.record,
            self.tags,
            ordinal,
            self.start_ms,
            self.end_ms,
            self.texts.join(" "),
        )
    }
}

fn make_chunk(
    record: &HistoryRecord,
    tags: &[String],
    ordinal: usize,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    text: String,
) -> ContextChunk {
    let id = format!("{}:{}", record.id, ordinal);
    let mut terms = extract_terms(&format!("{} {} {}", record.title, tags.join(" "), text));
    terms.sort();
    terms.dedup();
    ContextChunk {
        id,
        note_id: record.id.clone(),
        note_title: record.title.clone(),
        note_kind: record.kind,
        created_at: record.created_at.clone(),
        start_ms,
        end_ms,
        text: text.trim().to_string(),
        terms,
        tags: tags.to_vec(),
    }
}

pub fn extract_terms(input: &str) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for raw in input.split(|ch: char| !ch.is_alphanumeric() && ch != '-') {
        let token = raw.trim_matches('-').trim().to_ascii_lowercase();
        if token.len() < 3 || STOP_WORDS.contains(&token.as_str()) {
            continue;
        }
        *counts.entry(stem_token(&token)).or_default() += 1;
    }
    let mut terms: Vec<String> = counts
        .into_iter()
        .filter_map(|(term, count)| (count > 1 || term.len() > 4).then_some(term))
        .collect();
    terms.sort();
    terms
}

fn stem_token(token: &str) -> String {
    for suffix in ["ing", "ed", "es", "s"] {
        if token.len() > suffix.len() + 3 && token.ends_with(suffix) {
            return token[..token.len() - suffix.len()].to_string();
        }
    }
    token.to_string()
}

fn score_chunk(
    chunk: ContextChunk,
    vector: Vec<f32>,
    query_vector: &[f32],
    query_terms: &[String],
    now: chrono::DateTime<chrono::Utc>,
) -> ContextSearchResult {
    let vector_score = cosine_similarity(query_vector, &vector).max(0.0);
    let chunk_terms: HashSet<&str> = chunk.terms.iter().map(String::as_str).collect();
    let mut matched_terms: Vec<String> = query_terms
        .iter()
        .filter(|term| chunk_terms.contains(term.as_str()))
        .cloned()
        .collect();
    matched_terms.sort();
    matched_terms.dedup();
    let keyword_score = if query_terms.is_empty() {
        0.0
    } else {
        matched_terms.len() as f32 / query_terms.len() as f32
    };
    let tag_score = if chunk.tags.iter().any(|tag| {
        let tag = normalize_tag(tag);
        query_terms.iter().any(|term| term == &tag)
    }) {
        1.0
    } else {
        0.0
    };
    let recency_score = recency_score(&chunk.created_at, now);
    let total = (0.60 * vector_score)
        + (0.25 * keyword_score)
        + (0.05 * tag_score)
        + (0.10 * recency_score);
    ContextSearchResult {
        chunk_id: chunk.id,
        note_id: chunk.note_id,
        note_title: chunk.note_title,
        note_kind: chunk.note_kind,
        created_at: chunk.created_at,
        start_ms: chunk.start_ms,
        end_ms: chunk.end_ms,
        snippet: snippet(&chunk.text),
        matched_terms,
        score: ScoreBreakdown {
            vector: vector_score,
            keyword: keyword_score,
            tag: tag_score,
            recency: recency_score,
            total,
        },
    }
}

pub fn recency_score(created_at: &str, now: chrono::DateTime<chrono::Utc>) -> f32 {
    let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_at) else {
        return 0.0;
    };
    let age_days = now
        .signed_duration_since(created.with_timezone(&chrono::Utc))
        .num_days()
        .max(0) as f32;
    0.5_f32.powf(age_days / 90.0)
}

fn write_index(
    save_folder: &str,
    manifest: &ContextIndexManifest,
    chunks: &[ContextChunk],
    vectors: &[Vec<f32>],
) -> Result<()> {
    let dir = index_dir(save_folder);
    std::fs::create_dir_all(&dir).context("create context index directory")?;
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(manifest).context("serialize context index manifest")?,
    )
    .context("write context index manifest")?;

    let mut chunk_file =
        std::fs::File::create(dir.join("chunks.jsonl")).context("create chunks.jsonl")?;
    for chunk in chunks {
        serde_json::to_writer(&mut chunk_file, chunk).context("serialize context chunk")?;
        chunk_file.write_all(b"\n")?;
    }
    chunk_file.flush()?;

    let mut vector_file =
        std::fs::File::create(dir.join("vectors.f32")).context("create vectors.f32")?;
    for vector in vectors {
        for value in vector {
            vector_file.write_all(&value.to_le_bytes())?;
        }
    }
    vector_file.flush()?;
    Ok(())
}

fn find_files_named(root: &Path, file_name: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("read cache directory {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

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

fn read_index(
    save_folder: &str,
) -> Result<(ContextIndexManifest, Vec<ContextChunk>, Vec<Vec<f32>>)> {
    let dir = index_dir(save_folder);
    let manifest: ContextIndexManifest = serde_json::from_slice(
        &std::fs::read(dir.join("manifest.json")).context("read context index manifest")?,
    )
    .context("parse context index manifest")?;
    let chunks_file = std::fs::File::open(dir.join("chunks.jsonl")).context("open chunks.jsonl")?;
    let mut chunks = Vec::new();
    for line in std::io::BufReader::new(chunks_file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        chunks.push(serde_json::from_str::<ContextChunk>(&line).context("parse context chunk")?);
    }
    let raw_vectors = std::fs::read(dir.join("vectors.f32")).context("read vectors.f32")?;
    if manifest.embedding_dim == 0 {
        return Ok((manifest, chunks, Vec::new()));
    }
    let expected_bytes = chunks.len() * manifest.embedding_dim * std::mem::size_of::<f32>();
    if raw_vectors.len() != expected_bytes {
        return Err(anyhow!(
            "vectors.f32 has {} bytes, expected {}",
            raw_vectors.len(),
            expected_bytes
        ));
    }
    let vectors = raw_vectors
        .chunks_exact(manifest.embedding_dim * std::mem::size_of::<f32>())
        .map(|chunk| {
            chunk
                .chunks_exact(std::mem::size_of::<f32>())
                .map(|bytes| f32::from_le_bytes(bytes.try_into().expect("f32 byte chunk")))
                .collect::<Vec<f32>>()
        })
        .collect();
    Ok((manifest, chunks, vectors))
}

fn normalize(mut embedding: Vec<f32>) -> Vec<f32> {
    let norm = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut embedding {
            *value /= norm;
        }
    }
    embedding
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut a_norm = 0.0;
    let mut b_norm = 0.0;
    for (left, right) in a.iter().zip(b) {
        dot += left * right;
        a_norm += left * left;
        b_norm += right * right;
    }
    if a_norm == 0.0 || b_norm == 0.0 {
        0.0
    } else {
        dot / (a_norm.sqrt() * b_norm.sqrt())
    }
}

fn snippet(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= SNIPPET_MAX_CHARS {
        return collapsed;
    }
    let mut out = collapsed
        .chars()
        .take(SNIPPET_MAX_CHARS.saturating_sub(1))
        .collect::<String>();
    out.push('…');
    out
}

fn corpus_fingerprint(records: &[HistoryRecord]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for record in records {
        record.id.hash(&mut hasher);
        record.created_at.hash(&mut hasher);
        record.title.hash(&mut hasher);
        record.word_count.hash(&mut hasher);
        record.segments.len().hash(&mut hasher);
        record.written_content.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn normalize_tag(tag: &str) -> String {
    tag.trim().to_ascii_lowercase().replace(' ', "-")
}

fn first_start_ms(segments: &[Segment]) -> Option<i64> {
    segments.first().map(|segment| segment.start_ms)
}

fn last_end_ms(segments: &[Segment]) -> Option<i64> {
    segments.last().map(|segment| segment.end_ms)
}

fn timestamp_anchor(start_ms: Option<i64>, end_ms: Option<i64>) -> Option<String> {
    let start = start_ms?;
    let end = end_ms.unwrap_or(start);
    Some(format!("{}-{}", format_ms(start), format_ms(end)))
}

fn format_ms(ms: i64) -> String {
    let total_secs = (ms / 1000).max(0);
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}

const STOP_WORDS: &[&str] = &[
    "about", "after", "again", "also", "and", "are", "because", "been", "but", "can", "could",
    "did", "does", "for", "from", "had", "has", "have", "her", "him", "his", "how", "into", "its",
    "just", "like", "not", "our", "out", "she", "that", "the", "their", "them", "then", "there",
    "these", "they", "this", "those", "was", "were", "what", "when", "where", "which", "who",
    "why", "with", "would", "you", "your",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HistoryRecord, Segment};

    struct TestEmbeddingProvider;

    impl EmbeddingProvider for TestEmbeddingProvider {
        fn model_id(&self) -> &'static str {
            DEFAULT_MODEL_ID
        }

        fn embed_documents(&mut self, documents: &[String]) -> Result<Vec<Vec<f32>>> {
            Ok(documents.iter().map(|doc| fake_embedding(doc)).collect())
        }

        fn embed_query(&mut self, query: &str) -> Result<Vec<f32>> {
            Ok(fake_embedding(query))
        }
    }

    fn fake_embedding(text: &str) -> Vec<f32> {
        let mut vec = vec![0.0_f32; 8];
        for term in extract_terms(text) {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            term.hash(&mut hasher);
            let idx = (hasher.finish() as usize) % vec.len();
            vec[idx] += 1.0;
        }
        normalize(vec)
    }

    fn temp_folder() -> String {
        let dir =
            std::env::temp_dir().join(format!("scribefloat-context-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn transcript_chunks_keep_timestamp_ranges() {
        let mut rec = HistoryRecord::from_scribe(
            "Interview".into(),
            "small".into(),
            vec![
                Segment::new(0, 1_000, "Project alpha needs faster onboarding"),
                Segment::new(1_000, 2_000, "Stakeholder Sarah asked about rollout timing"),
            ],
            Vec::new(),
            false,
            false,
            None,
            None,
            None,
        );
        rec.id = "note-1".into();
        let chunks = chunk_records("", &[rec]);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_ms, Some(0));
        assert_eq!(chunks[0].end_ms, Some(2_000));
        assert!(chunks[0].terms.contains(&"project".to_string()));
    }

    #[test]
    fn written_chunks_split_by_headings_and_paragraphs() {
        let mut rec = HistoryRecord::from_written("Plan".into());
        rec.id = "written-1".into();
        rec.written_content = Some("# Alpha\nFirst paragraph.\n\n# Beta\nSecond paragraph.".into());
        let chunks = chunk_records("", &[rec]);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.contains("# Alpha"));
        assert!(chunks[1].text.contains("# Beta"));
    }

    #[test]
    fn short_dictate_note_is_one_chunk() {
        let seg = Segment::new(0, 1_000, "remember the onboarding decision");
        let mut rec =
            HistoryRecord::from_dictate(&[seg], "remember the onboarding decision", "tiny".into());
        rec.id = "dictate-1".into();
        let chunks = chunk_records("", &[rec]);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_ms, Some(0));
    }

    #[test]
    fn term_extraction_stems_and_removes_stop_words() {
        let terms = extract_terms("The onboarding designs are designing a better onboarding flow");
        assert!(terms.contains(&"onboard".to_string()));
        assert!(!terms.contains(&"the".to_string()));
    }

    #[test]
    fn file_sha256_hex_matches_known_vector() {
        let folder = temp_folder();
        let path = PathBuf::from(folder).join("abc.txt");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            file_sha256_hex(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn bge_cache_verification_rejects_wrong_model_hash() {
        let folder = temp_folder();
        let path = PathBuf::from(&folder).join(BGE_MODEL_FILE);
        std::fs::write(&path, b"not the bge model").unwrap();
        let err = verify_bge_model_cache(Path::new(&folder)).unwrap_err();
        assert!(err.to_string().contains("SHA-256 mismatch"));
    }

    #[test]
    fn recency_score_prefers_recent_dates() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-11T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let recent = recency_score("2026-07-10T00:00:00Z", now);
        let old = recency_score("2026-01-10T00:00:00Z", now);
        assert!(recent > old);
    }

    #[test]
    fn build_search_and_export_roundtrip() {
        let folder = temp_folder();
        let history = HistoryService::new();
        let rec = HistoryRecord::from_dictate(
            &[Segment::new(0, 1_000, "Project alpha onboarding decision")],
            "Project alpha onboarding decision",
            "tiny".into(),
        );
        history.append(&folder, rec).unwrap();
        let mut provider = TestEmbeddingProvider;
        let manifest = build_index(&history, &folder, &mut provider).unwrap();
        assert_eq!(manifest.chunk_count, 1);

        let mut options = SearchOptions::new("onboarding decision".into());
        options.limit = 5;
        let results = search_index(&folder, &mut provider, &options).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].score.total > 0.0);

        let out = PathBuf::from(&folder).join("context.md");
        export_context_pack(&folder, &mut provider, &options, &out).unwrap();
        let markdown = std::fs::read_to_string(out).unwrap();
        assert!(markdown.contains("# Context Pack"));
        assert!(markdown.contains("Project alpha"));
    }
}
