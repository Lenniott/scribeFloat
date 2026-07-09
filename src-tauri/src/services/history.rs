use crate::services::note_sidecar;
use crate::services::voice_embeddings::VoiceEmbeddingStore;
use crate::types::{HistoryListItem, HistoryRecord};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Owns the canonical structured record store: `{save_folder}/history.jsonl`.
///
/// Capture lifecycle events append to the log; editor title/body use [`note_sidecar`].
/// See `docs/engineering/history-storage.md`.
///
/// Mirrors `OutputService`'s stateless-with-folder style: the save folder is passed per
/// call. When it changes the in-memory cache is reloaded for the new folder (no migration).
pub struct HistoryService {
    inner: Mutex<HistoryInner>,
    /// How voice embeddings rest on disk. Plaintext until [`set_embedding_store`]
    /// injects the store chosen at startup.
    embedding_store: Mutex<Arc<VoiceEmbeddingStore>>,
}

struct HistoryInner {
    /// The folder the cache currently reflects. `None` until first load.
    save_folder: Option<String>,
    /// Live, deduped records (may include tombstones with `deleted = true`).
    records: Vec<HistoryRecord>,
    /// id → index into `records` for last-writer-wins upserts.
    index: HashMap<String, usize>,
}

impl HistoryService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(HistoryInner {
                save_folder: None,
                records: Vec::new(),
                index: HashMap::new(),
            }),
            embedding_store: Mutex::new(VoiceEmbeddingStore::plaintext()),
        })
    }

    pub fn set_embedding_store(&self, store: Arc<VoiceEmbeddingStore>) {
        *self
            .embedding_store
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = store;
    }

    fn embedding_store(&self) -> Arc<VoiceEmbeddingStore> {
        self.embedding_store
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn store_path(save_folder: &str) -> PathBuf {
        PathBuf::from(save_folder).join("history.jsonl")
    }

    /// Ensure the in-memory cache reflects `save_folder`, reloading from disk if the folder
    /// changed (or this is the first access).
    fn ensure_loaded(&self, inner: &mut HistoryInner, save_folder: &str) -> Result<()> {
        if inner.save_folder.as_deref() == Some(save_folder) {
            return Ok(());
        }
        inner.records.clear();
        inner.index.clear();
        inner.save_folder = Some(save_folder.to_string());

        let path = Self::store_path(save_folder);
        if !path.exists() {
            return Ok(());
        }
        let raw = std::fs::read_to_string(&path).context("read history.jsonl")?;
        let lines: Vec<&str> = raw.lines().collect();
        let last = lines.len().saturating_sub(1);
        let store = self.embedding_store();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryRecord>(line) {
                Ok(mut record) => {
                    // Transcript data stays usable without embeddings, so a failed
                    // unseal degrades the record instead of dropping it.
                    if let Err(err) = store.unseal_record(&mut record) {
                        tracing::warn!(
                            id = %record.id,
                            error = %err,
                            "voice embeddings could not be decrypted"
                        );
                    }
                    Self::apply_record(inner, record)
                }
                Err(e) => {
                    // A corrupt *trailing* line is a partial append after a crash — ignore it.
                    // A corrupt middle line is unexpected; log and skip (matches ConfigService).
                    if i != last {
                        tracing::warn!(line = i, error = %e, "skipping corrupt history.jsonl line");
                    }
                }
            }
        }
        for record in inner.records.iter_mut() {
            note_sidecar::hydrate_record(save_folder, record);
        }
        Ok(())
    }

    /// Upsert a record into the in-memory structure (last-writer-wins by id).
    fn apply_record(inner: &mut HistoryInner, record: HistoryRecord) {
        if let Some(&idx) = inner.index.get(&record.id) {
            inner.records[idx] = record;
        } else {
            let idx = inner.records.len();
            inner.index.insert(record.id.clone(), idx);
            inner.records.push(record);
        }
    }

    /// Append a single record line to `history.jsonl` (creating the folder/file if needed).
    fn append_line(
        save_folder: &str,
        record: &HistoryRecord,
        store: &VoiceEmbeddingStore,
    ) -> Result<()> {
        let path = Self::store_path(save_folder);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create save folder for history store")?;
        }
        let mut disk_record = record.clone();
        store.seal_record(&mut disk_record)?;
        let line = serde_json::to_string(&disk_record).context("serialize history record")?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .context("open history.jsonl for append")?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    }

    /// Append a new record. Returns the record id.
    pub fn append(&self, save_folder: &str, record: HistoryRecord) -> Result<String> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let id = record.id.clone();
        let store = self.embedding_store();
        Self::append_line(save_folder, &record, &store)?;
        Self::apply_record(&mut inner, record);
        Ok(id)
    }

    /// Set/replace the exported markdown path for a record (log-structured update).
    pub fn set_markdown_path(&self, save_folder: &str, id: &str, md_path: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(());
        };
        let mut updated = inner.records[idx].clone();
        updated.markdown_path = Some(md_path.to_string());
        let store = self.embedding_store();
        Self::append_line(save_folder, &updated, &store)?;
        inner.records[idx] = updated;
        Ok(())
    }

    /// Rename a transcript-local speaker group and cascade that label to its
    /// chunks and rendered speaker blocks. This is a log-structured update.
    pub fn rename_session_speaker(
        &self,
        save_folder: &str,
        id: &str,
        session_speaker_id: &str,
        label: &str,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(());
        };

        let mut updated = inner.records[idx].clone();
        let Some(speaker) = updated
            .session_speakers
            .iter_mut()
            .find(|speaker| speaker.session_speaker_id == session_speaker_id)
        else {
            return Ok(());
        };
        speaker.label = label.to_string();
        speaker.user_confirmed = true;

        let chunk_ids: HashSet<String> = updated
            .speaker_chunks
            .iter_mut()
            .filter(|chunk| chunk.cluster_id.as_deref() == Some(session_speaker_id))
            .map(|chunk| {
                chunk.label = label.to_string();
                chunk.id.clone()
            })
            .collect();

        for block in &mut updated.speaker_blocks {
            if block
                .chunk_id
                .as_ref()
                .is_some_and(|chunk_id| chunk_ids.contains(chunk_id))
            {
                block.label = label.to_string();
            }
        }

        let store = self.embedding_store();
        Self::append_line(save_folder, &updated, &store)?;
        inner.records[idx] = updated;
        Ok(())
    }

    /// Remove biometric voice vectors from a note while keeping transcript text,
    /// labels, timing, quality scores, cuts, chunks, and session speaker groups.
    pub fn remove_voice_embeddings(&self, save_folder: &str, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(());
        };

        let mut updated = inner.records[idx].clone();
        strip_voice_embeddings(&mut updated);
        let store = self.embedding_store();
        Self::append_line(save_folder, &updated, &store)?;
        inner.records[idx] = updated;
        Ok(())
    }

    /// Remove biometric voice vectors from all live notes in the save folder.
    pub fn remove_all_voice_embeddings(&self, save_folder: &str) -> Result<usize> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;

        let mut changed = 0usize;
        let live_indexes: Vec<usize> = inner
            .records
            .iter()
            .enumerate()
            .filter_map(|(idx, record)| (!record.deleted).then_some(idx))
            .collect();

        for idx in live_indexes {
            let mut updated = inner.records[idx].clone();
            if strip_voice_embeddings(&mut updated) {
                let store = self.embedding_store();
                Self::append_line(save_folder, &updated, &store)?;
                inner.records[idx] = updated;
                changed += 1;
            }
        }

        Ok(changed)
    }

    /// Update written body text — overwrites `{save_folder}/.notes/{id}/written.md` in place.
    /// Does not append to `history.jsonl` (high-frequency editor autosave).
    pub fn update_written_content(&self, save_folder: &str, id: &str, content: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(());
        };
        note_sidecar::write_written(save_folder, id, content)?;
        let mut updated = inner.records[idx].clone();
        updated.written_content = Some(content.to_string());
        updated.word_count = content.split_whitespace().count();
        inner.records[idx] = updated;
        Ok(())
    }

    /// Update display title — overwrites `{save_folder}/.notes/{id}/meta.json` in place.
    /// Does not append to `history.jsonl` (editor metadata, not a new capture event).
    pub fn update_title(&self, save_folder: &str, id: &str, title: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(());
        };
        if inner.records[idx].title == title {
            return Ok(());
        }
        note_sidecar::write_meta_title(save_folder, id, title)?;
        inner.records[idx].title = title.to_string();
        Ok(())
    }

    /// Attach a transcription pass to an existing note (log-structured update).
    /// Timeline shifting, duration, and word count are owned by
    /// `HistoryRecord::attach_transcript`; this method only persists the result.
    pub fn attach_transcript(
        &self,
        save_folder: &str,
        id: &str,
        attachment: crate::types::TranscriptAttachment,
    ) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(());
        };
        let mut updated = inner.records[idx].clone();
        updated.attach_transcript(attachment);
        let store = self.embedding_store();
        Self::append_line(save_folder, &updated, &store)?;
        inner.records[idx] = updated;
        Ok(())
    }

    /// Tombstone a record. Returns the record as it was before deletion (so the caller can
    /// remove its derived artifacts), or `None` if the id is unknown or already deleted.
    pub fn delete(&self, save_folder: &str, id: &str) -> Result<Option<HistoryRecord>> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            return Ok(None);
        };
        if inner.records[idx].deleted {
            return Ok(None);
        }
        let before = inner.records[idx].clone();
        let mut tombstone = before.clone();
        tombstone.deleted = true;
        let store = self.embedding_store();
        Self::append_line(save_folder, &tombstone, &store)?;
        inner.records[idx] = tombstone;
        note_sidecar::remove_note_dir(save_folder, id);
        Ok(Some(before))
    }

    /// All live (non-deleted) records, newest-first by `created_at`.
    /// Prefer [`list_summaries`](Self::list_summaries) for History UI; this clones full segment payloads.
    #[allow(dead_code)]
    pub fn list(&self, save_folder: &str) -> Result<Vec<HistoryRecord>> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let mut out: Vec<HistoryRecord> = inner
            .records
            .iter()
            .filter(|r| !r.deleted)
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(out)
    }

    /// Lightweight list projection for History UI — no segment payload in the return value.
    pub fn list_summaries(&self, save_folder: &str) -> Result<Vec<HistoryListItem>> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let mut out: Vec<HistoryListItem> = inner
            .records
            .iter()
            .filter(|r| !r.deleted)
            .map(HistoryRecord::to_list_item)
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(out)
    }

    /// Fetch a single live record by id.
    pub fn get(&self, save_folder: &str, id: &str) -> Result<Option<HistoryRecord>> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        Ok(inner
            .index
            .get(id)
            .map(|&idx| inner.records[idx].clone())
            .filter(|r| !r.deleted))
    }

    /// Rewrite `history.jsonl` to just the live set (drops tombstones and superseded lines).
    /// Startup-only; atomic via temp file + rename. Non-fatal on failure.
    pub fn compact(&self, save_folder: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let live: Vec<&HistoryRecord> = inner.records.iter().filter(|r| !r.deleted).collect();

        let path = Self::store_path(save_folder);
        if !path.exists() && live.is_empty() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create save folder for history compaction")?;
        }
        let tmp = path.with_extension("jsonl.tmp");
        {
            let mut file = std::fs::File::create(&tmp).context("create history.jsonl.tmp")?;
            let store = self.embedding_store();
            for record in &live {
                let mut disk_record = (*record).clone();
                store.seal_record(&mut disk_record)?;
                let line =
                    serde_json::to_string(&disk_record).context("serialize history record")?;
                file.write_all(line.as_bytes())?;
                file.write_all(b"\n")?;
            }
            file.flush()?;
            file.sync_all()?;
        }
        std::fs::rename(&tmp, &path).context("rename history.jsonl.tmp")?;
        Ok(())
    }
}

/// Remove only embedding vectors. Human-readable transcript data stays usable.
pub fn strip_voice_embeddings(record: &mut HistoryRecord) -> bool {
    let mut changed = false;

    for chunk in &mut record.speaker_chunks {
        if chunk.embedding.take().is_some() {
            changed = true;
        }
        if chunk.encrypted_embedding.take().is_some() {
            changed = true;
        }
    }

    for speaker in &mut record.session_speakers {
        if !speaker.centroid_embedding.is_empty() {
            speaker.centroid_embedding.clear();
            changed = true;
        }
        if speaker.encrypted_centroid_embedding.take().is_some() {
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::note_sidecar;
    use crate::services::voice_crypto::{StaticVoiceCryptoKeyProvider, VoiceCryptoService};
    use crate::types::{HistoryKind, Segment};

    fn temp_folder() -> String {
        let dir =
            std::env::temp_dir().join(format!("scribefloat-history-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir.to_string_lossy().to_string()
    }

    fn record(text: &str) -> HistoryRecord {
        let segs = vec![Segment {
            start_ms: 0,
            end_ms: 1_000,
            text: text.to_string(),
            source: None,
        }];
        HistoryRecord::from_dictate(&segs, text, "tiny".to_string())
    }

    fn test_store() -> Arc<VoiceEmbeddingStore> {
        VoiceEmbeddingStore::encrypted(VoiceCryptoService::new(Arc::new(
            StaticVoiceCryptoKeyProvider::new(11),
        )))
    }

    #[test]
    fn list_summaries_matches_list_ids_without_cloning_segments_in_api() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut rec = record("short");
        rec.segments = (0..200)
            .map(|i| Segment {
                start_ms: i * 1000,
                end_ms: (i + 1) * 1000,
                text: format!("word{i} "),
                source: None,
            })
            .collect();
        rec.word_count = 200;
        let id = svc.append(&folder, rec).expect("append");
        let summaries = svc.list_summaries(&folder).expect("summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, id);
        assert_eq!(summaries[0].word_count, 200);
        let full = svc.list(&folder).expect("list");
        assert_eq!(full[0].segments.len(), 200);
    }

    #[test]
    fn append_then_list_roundtrip() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let id = svc.append(&folder, record("hello")).expect("append");
        let list = svc.list(&folder).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);

        // Reload from disk in a fresh service instance.
        let svc2 = HistoryService::new();
        let list2 = svc2.list(&folder).expect("list2");
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].id, id);
    }

    #[test]
    fn list_is_newest_first() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut a = record("first");
        a.created_at = "2026-01-01T00:00:00Z".to_string();
        let mut b = record("second");
        b.created_at = "2026-02-01T00:00:00Z".to_string();
        svc.append(&folder, a).unwrap();
        svc.append(&folder, b).unwrap();
        let list = svc.list(&folder).unwrap();
        assert_eq!(list[0].created_at, "2026-02-01T00:00:00Z");
    }

    #[test]
    fn set_markdown_path_last_writer_wins() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let id = svc.append(&folder, record("note")).unwrap();
        svc.set_markdown_path(&folder, &id, "/save/note.md")
            .unwrap();

        let fresh = HistoryService::new();
        let got = fresh.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.markdown_path.as_deref(), Some("/save/note.md"));
    }

    #[test]
    fn rename_session_speaker_cascades_to_chunks_and_blocks() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut rec = record("hello");
        rec.session_speakers = vec![crate::types::SessionSpeaker {
            session_speaker_id: "speaker-1".into(),
            label: "Speaker A".into(),
            centroid_embedding: vec![1.0, 0.0],
            encrypted_centroid_embedding: None,
            clean_chunk_ids: vec!["chunk-0001".into()],
            start_ms: 0,
            end_ms: 2_000,
            duration_ms: 2_000,
            radius: 0.0,
            quality_score: 0.9,
            user_confirmed: false,
        }];
        rec.speaker_chunks = vec![crate::types::SpeakerChunk {
            id: "chunk-0001".into(),
            start_ms: 0,
            end_ms: 2_000,
            label: "Speaker A".into(),
            cluster_id: Some("speaker-1".into()),
            matched_profile: None,
            embedding: Some(vec![1.0, 0.0]),
            encrypted_embedding: None,
            audio_duration_s: 2.0,
            vad_purity: 1.0,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: None,
            margin: None,
        }];
        rec.speaker_blocks = vec![crate::types::SpeakerBlock {
            label: "Speaker A".into(),
            start_ms: Some(0),
            end_ms: Some(2_000),
            text: "hello".into(),
            chunk_id: Some("chunk-0001".into()),
        }];
        let id = svc.append(&folder, rec).expect("append");

        svc.rename_session_speaker(&folder, &id, "speaker-1", "Gilgamesh")
            .expect("rename speaker");

        let got = svc.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.session_speakers[0].label, "Gilgamesh");
        assert!(got.session_speakers[0].user_confirmed);
        assert_eq!(got.speaker_chunks[0].label, "Gilgamesh");
        assert_eq!(got.speaker_blocks[0].label, "Gilgamesh");
    }

    #[test]
    fn remove_voice_embeddings_keeps_transcript_speaker_evidence() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut rec = record("hello");
        rec.session_speakers = vec![crate::types::SessionSpeaker {
            session_speaker_id: "speaker-1".into(),
            label: "Speaker A".into(),
            centroid_embedding: vec![1.0, 0.0],
            encrypted_centroid_embedding: None,
            clean_chunk_ids: vec!["chunk-0001".into()],
            start_ms: 0,
            end_ms: 2_000,
            duration_ms: 2_000,
            radius: 0.0,
            quality_score: 0.9,
            user_confirmed: true,
        }];
        rec.speaker_chunks = vec![crate::types::SpeakerChunk {
            id: "chunk-0001".into(),
            start_ms: 0,
            end_ms: 2_000,
            label: "Speaker A".into(),
            cluster_id: Some("speaker-1".into()),
            matched_profile: None,
            embedding: Some(vec![1.0, 0.0]),
            encrypted_embedding: None,
            audio_duration_s: 2.0,
            vad_purity: 1.0,
            rms_energy: 0.1,
            clipping: false,
            profile_score: Some(0.8),
            session_score: None,
            margin: None,
        }];
        let id = svc.append(&folder, rec).expect("append");

        svc.remove_voice_embeddings(&folder, &id)
            .expect("remove embeddings");

        let got = svc.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.speaker_chunks[0].label, "Speaker A");
        assert_eq!(
            got.speaker_chunks[0].cluster_id.as_deref(),
            Some("speaker-1")
        );
        assert_eq!(got.speaker_chunks[0].embedding, None);
        assert_eq!(got.speaker_chunks[0].profile_score, Some(0.8));
        assert_eq!(got.session_speakers[0].label, "Speaker A");
        assert!(got.session_speakers[0].centroid_embedding.is_empty());
        assert_eq!(got.session_speakers[0].clean_chunk_ids, vec!["chunk-0001"]);
        assert!(got.session_speakers[0].user_confirmed);
    }

    #[test]
    fn remove_all_voice_embeddings_only_counts_changed_records() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut with_embedding = record("hello");
        with_embedding.speaker_chunks = vec![crate::types::SpeakerChunk {
            id: "chunk-0001".into(),
            start_ms: 0,
            end_ms: 2_000,
            label: "Speaker A".into(),
            cluster_id: Some("speaker-1".into()),
            matched_profile: None,
            embedding: Some(vec![1.0, 0.0]),
            encrypted_embedding: None,
            audio_duration_s: 2.0,
            vad_purity: 1.0,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: None,
            margin: None,
        }];
        let changed_id = svc.append(&folder, with_embedding).expect("append changed");
        let unchanged_id = svc.append(&folder, record("plain")).expect("append plain");

        let changed = svc
            .remove_all_voice_embeddings(&folder)
            .expect("remove all embeddings");

        assert_eq!(changed, 1);
        assert_eq!(
            svc.get(&folder, &changed_id)
                .unwrap()
                .expect("changed present")
                .speaker_chunks[0]
                .embedding,
            None
        );
        assert!(svc
            .get(&folder, &unchanged_id)
            .unwrap()
            .expect("unchanged present")
            .speaker_chunks
            .is_empty());
    }

    #[test]
    fn configured_crypto_encrypts_history_embeddings_at_rest_and_decrypts_on_load() {
        let folder = temp_folder();
        let store = test_store();
        let svc = HistoryService::new();
        svc.set_embedding_store(Arc::clone(&store));
        let mut rec = record("encrypted");
        rec.speaker_chunks = vec![crate::types::SpeakerChunk {
            id: "chunk-0001".into(),
            start_ms: 0,
            end_ms: 2_000,
            label: "Speaker A".into(),
            cluster_id: Some("speaker-1".into()),
            matched_profile: None,
            embedding: Some(vec![1.0, 0.0]),
            encrypted_embedding: None,
            audio_duration_s: 2.0,
            vad_purity: 1.0,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: None,
            margin: None,
        }];
        rec.session_speakers = vec![crate::types::SessionSpeaker {
            session_speaker_id: "speaker-1".into(),
            label: "Speaker A".into(),
            centroid_embedding: vec![0.0, 1.0],
            encrypted_centroid_embedding: None,
            clean_chunk_ids: vec!["chunk-0001".into()],
            start_ms: 0,
            end_ms: 2_000,
            duration_ms: 2_000,
            radius: 0.0,
            quality_score: 0.9,
            user_confirmed: false,
        }];
        let id = svc.append(&folder, rec).expect("append encrypted");

        let raw = std::fs::read_to_string(HistoryService::store_path(&folder)).unwrap();
        assert!(raw.contains("encrypted_embedding"));
        assert!(raw.contains("encrypted_centroid_embedding"));
        assert!(!raw.contains(r#""embedding":[1.0,0.0]"#));
        assert!(!raw.contains(r#""centroid_embedding":[0.0,1.0]"#));

        let fresh = HistoryService::new();
        fresh.set_embedding_store(store);
        let got = fresh.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.speaker_chunks[0].embedding, Some(vec![1.0, 0.0]));
        assert_eq!(got.session_speakers[0].centroid_embedding, vec![0.0, 1.0]);
    }

    #[test]
    fn remove_voice_embeddings_clears_encrypted_history_vectors_too() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        svc.set_embedding_store(test_store());
        let mut rec = record("encrypted");
        rec.speaker_chunks = vec![crate::types::SpeakerChunk {
            id: "chunk-0001".into(),
            start_ms: 0,
            end_ms: 2_000,
            label: "Speaker A".into(),
            cluster_id: Some("speaker-1".into()),
            matched_profile: None,
            embedding: Some(vec![1.0, 0.0]),
            encrypted_embedding: None,
            audio_duration_s: 2.0,
            vad_purity: 1.0,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: None,
            margin: None,
        }];
        let id = svc.append(&folder, rec).expect("append encrypted");

        svc.remove_voice_embeddings(&folder, &id)
            .expect("remove encrypted embeddings");

        let raw = std::fs::read_to_string(HistoryService::store_path(&folder)).unwrap();
        let last_line = raw.lines().last().unwrap();
        assert!(!last_line.contains("encrypted_embedding"));
        let got = svc.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.speaker_chunks[0].label, "Speaker A");
        assert_eq!(got.speaker_chunks[0].embedding, None);
    }

    #[test]
    fn delete_tombstones_and_returns_record() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let id = svc.append(&folder, record("doomed")).unwrap();
        let removed = svc.delete(&folder, &id).unwrap().expect("returns record");
        assert_eq!(removed.id, id);
        assert!(svc.list(&folder).unwrap().is_empty());
        // Second delete is idempotent.
        assert!(svc.delete(&folder, &id).unwrap().is_none());

        // Tombstone survives reload.
        let fresh = HistoryService::new();
        assert!(fresh.list(&folder).unwrap().is_empty());
    }

    #[test]
    fn delete_removes_sidecar_directory() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Sidecar".into());
        let id = svc.append(&folder, rec).expect("append");
        svc.update_written_content(&folder, &id, "body").unwrap();
        svc.update_title(&folder, &id, "renamed").unwrap();
        assert!(note_sidecar::note_dir(&folder, &id).exists());

        svc.delete(&folder, &id).unwrap();
        assert!(!note_sidecar::note_dir(&folder, &id).exists());
    }

    #[test]
    fn update_written_content_roundtrips() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Draft".into());
        let id = svc.append(&folder, rec).expect("append");

        svc.update_written_content(&folder, &id, "# Hello\n\nWorld content here")
            .expect("update");

        let fresh = HistoryService::new();
        let got = fresh.get(&folder, &id).unwrap().expect("present");
        assert_eq!(
            got.written_content.as_deref(),
            Some("# Hello\n\nWorld content here")
        );
        // word_count recomputed from content ("#", "Hello", "World", "content", "here")
        assert_eq!(got.word_count, 5);
    }

    #[test]
    fn update_written_content_does_not_append_jsonl_lines() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Draft".into());
        let id = svc.append(&folder, rec).expect("append");
        let path = HistoryService::store_path(&folder);
        let lines_after_create = std::fs::read_to_string(&path).unwrap().lines().count();

        for i in 0..5 {
            svc.update_written_content(&folder, &id, &format!("edit {i}"))
                .expect("update");
        }

        let lines_after_edits = std::fs::read_to_string(&path).unwrap().lines().count();
        assert_eq!(
            lines_after_create, lines_after_edits,
            "written autosave must not append history.jsonl lines"
        );
        assert!(note_sidecar::written_path(&folder, &id).exists());
    }

    #[test]
    fn update_title_roundtrips() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Old Title".into());
        let id_val = rec.id.clone();
        svc.append(&folder, rec).expect("append");

        svc.update_title(&folder, &id_val, "new title")
            .expect("update title");

        let fresh = HistoryService::new();
        let got = fresh.get(&folder, &id_val).unwrap().expect("present");
        assert_eq!(got.title, "new title");
        assert_eq!(got.id, id_val);
        assert!(note_sidecar::meta_path(&folder, &id_val).exists());
    }

    #[test]
    fn update_title_does_not_append_jsonl_lines() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Old Title".into());
        let id = svc.append(&folder, rec).expect("append");
        let path = HistoryService::store_path(&folder);
        let lines_after_create = std::fs::read_to_string(&path).unwrap().lines().count();

        for title in ["alpha", "beta", "gamma"] {
            svc.update_title(&folder, &id, title).expect("update title");
        }

        let lines_after_edits = std::fs::read_to_string(&path).unwrap().lines().count();
        assert_eq!(
            lines_after_create, lines_after_edits,
            "title edits must not append history.jsonl lines"
        );
    }

    #[test]
    fn attach_transcript_roundtrips() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Draft".into());
        let id = svc.append(&folder, rec).expect("append");
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "Hello".into(),
                source: None,
            },
            Segment {
                start_ms: 1_000,
                end_ms: 2_500,
                text: "world".into(),
                source: None,
            },
        ];

        svc.attach_transcript(
            &folder,
            &id,
            crate::types::TranscriptAttachment {
                segments,
                model: "base".into(),
                ..Default::default()
            },
        )
        .expect("attach transcript");

        let fresh = HistoryService::new();
        let got = fresh.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.segments.len(), 2);
        assert!(got.duration_ms > 0);
        assert_eq!(got.model, "base");
    }

    #[test]
    fn attach_transcript_appends_when_called_twice() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let rec = crate::types::HistoryRecord::from_written("Draft".into());
        let id = svc.append(&folder, rec).expect("append");

        svc.attach_transcript(
            &folder,
            &id,
            crate::types::TranscriptAttachment {
                segments: vec![Segment {
                    start_ms: 0,
                    end_ms: 1_000,
                    text: "first".into(),
                    source: None,
                }],
                notes: vec![crate::types::Note {
                    id: "note-1".into(),
                    text: "marker".into(),
                    recorded_at_ms: 500,
                }],
                model: "base".into(),
                ..Default::default()
            },
        )
        .expect("first attach");

        svc.attach_transcript(
            &folder,
            &id,
            crate::types::TranscriptAttachment {
                segments: vec![Segment {
                    start_ms: 0,
                    end_ms: 2_000,
                    text: "second".into(),
                    source: None,
                }],
                speaker_change_cuts: vec![crate::types::SpeakerChangeCut {
                    time_s: 0.5,
                    end_s: 0.5,
                    score: 1.5,
                    reasons: [crate::types::CutReason::Pitch].into_iter().collect(),
                }],
                session_speakers: vec![crate::types::SessionSpeaker {
                    session_speaker_id: "speaker-1".into(),
                    label: "Speaker A".into(),
                    centroid_embedding: vec![1.0, 0.0],
                    encrypted_centroid_embedding: None,
                    clean_chunk_ids: vec!["chunk-0001".into()],
                    start_ms: 0,
                    end_ms: 2_000,
                    duration_ms: 2_000,
                    radius: 0.0,
                    quality_score: 0.9,
                    user_confirmed: false,
                }],
                notes: vec![crate::types::Note {
                    id: "note-2".into(),
                    text: "second marker".into(),
                    recorded_at_ms: 250,
                }],
                model: "base".into(),
                ..Default::default()
            },
        )
        .expect("second attach");

        let got = svc.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.segments.len(), 2);
        assert_eq!(got.segments[0].text, "first");
        assert_eq!(got.segments[1].text, "second");
        assert_eq!(got.segments[1].start_ms, 1_000);
        assert_eq!(got.segments[1].end_ms, 3_000);
        assert_eq!(got.notes.len(), 2);
        assert_eq!(got.notes[1].recorded_at_ms, 1_250);
        assert_eq!(got.duration_ms, 3_000);
        // Cut attached in the second recording shifts by the 1 s offset.
        assert_eq!(got.speaker_change_cuts.len(), 1);
        assert!((got.speaker_change_cuts[0].time_s - 1.5).abs() < 1e-6);
        assert_eq!(got.session_speakers.len(), 1);
        assert_eq!(got.session_speakers[0].start_ms, 1_000);
        assert_eq!(got.session_speakers[0].end_ms, 3_000);
    }

    #[test]
    fn corrupt_trailing_line_ignored_corrupt_middle_skipped() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        svc.append(&folder, record("good-one")).unwrap();
        // Manually append a corrupt middle line then a good line, then a corrupt trailing line.
        let path = HistoryService::store_path(&folder);
        let good = serde_json::to_string(&record("good-two")).unwrap();
        let mut content = std::fs::read_to_string(&path).unwrap();
        content.push_str("{ this is not json }\n");
        content.push_str(&good);
        content.push('\n');
        content.push_str("{ partial trailing");
        std::fs::write(&path, content).unwrap();

        let fresh = HistoryService::new();
        let list = fresh.list(&folder).unwrap();
        // good-one + good-two survive; both corrupt lines dropped.
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn compact_drops_tombstones_and_superseded() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let keep = svc.append(&folder, record("keep")).unwrap();
        let drop_id = svc.append(&folder, record("drop")).unwrap();
        svc.set_markdown_path(&folder, &keep, "/save/keep.md")
            .unwrap();
        svc.delete(&folder, &drop_id).unwrap();

        svc.compact(&folder).unwrap();

        // Raw file now has exactly one line (the surviving record).
        let path = HistoryService::store_path(&folder);
        let raw = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 1);

        let fresh = HistoryService::new();
        let list = fresh.list(&folder).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, keep);
        assert_eq!(list[0].markdown_path.as_deref(), Some("/save/keep.md"));
    }

    #[test]
    fn save_folder_change_reloads_cache() {
        let folder_a = temp_folder();
        let folder_b = temp_folder();
        let svc = HistoryService::new();
        svc.append(&folder_a, record("in-a")).unwrap();
        assert_eq!(svc.list(&folder_a).unwrap().len(), 1);
        // Switching folders reloads — folder B is empty.
        assert_eq!(svc.list(&folder_b).unwrap().len(), 0);
        // Switching back reloads A.
        assert_eq!(svc.list(&folder_a).unwrap().len(), 1);
    }

    #[test]
    fn concurrent_appends_produce_clean_lines() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        std::thread::scope(|s| {
            for i in 0..16 {
                let svc = &svc;
                let folder = folder.clone();
                s.spawn(move || {
                    svc.append(&folder, record(&format!("rec-{i}"))).unwrap();
                });
            }
        });
        let path = HistoryService::store_path(&folder);
        let raw = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 16);
        for line in lines {
            serde_json::from_str::<HistoryRecord>(line).expect("each line parses cleanly");
        }
        assert_eq!(svc.list(&folder).unwrap().len(), 16);
        assert_eq!(svc.list(&folder).unwrap()[0].kind, HistoryKind::Dictate);
    }
}
