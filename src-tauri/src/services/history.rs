use crate::services::note_sidecar;
use crate::types::{HistoryListItem, HistoryRecord};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Owns the canonical structured record store: `{save_folder}/history.jsonl`.
///
/// Capture lifecycle events append to the log; editor title/body use [`note_sidecar`].
///
/// Mirrors `OutputService`'s stateless-with-folder style: the save folder is passed per
/// call. When it changes the in-memory cache is reloaded for the new folder (no migration).
pub struct HistoryService {
    inner: Mutex<HistoryInner>,
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
        })
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
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryRecord>(line) {
                Ok(record) => Self::apply_record(inner, record),
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
    fn append_line(save_folder: &str, record: &HistoryRecord) -> Result<()> {
        let path = Self::store_path(save_folder);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("create save folder for history store")?;
        }
        let line = serde_json::to_string(record).context("serialize history record")?;
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
        Self::append_line(save_folder, &record)?;
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
        Self::append_line(save_folder, &updated)?;
        inner.records[idx] = updated;
        Ok(())
    }

    /// Rename a speaker label across one note and persist the rewritten record.
    /// Errors when the note is unknown or nothing carried `from_label`.
    pub fn relabel_speaker(
        &self,
        save_folder: &str,
        id: &str,
        from_label: &str,
        to_label: &str,
    ) -> Result<HistoryRecord> {
        let mut inner = self.inner.lock().unwrap();
        self.ensure_loaded(&mut inner, save_folder)?;
        let Some(&idx) = inner.index.get(id) else {
            anyhow::bail!("note `{id}` not found");
        };

        let mut updated = inner.records[idx].clone();
        if relabel_speaker_blocks(&mut updated, from_label, to_label) == 0 {
            anyhow::bail!("note `{id}` has no speaker labeled `{from_label}`");
        }

        Self::append_line(save_folder, &updated)?;
        inner.records[idx] = updated.clone();
        Ok(updated)
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
        Self::append_line(save_folder, &updated)?;
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
        Self::append_line(save_folder, &tombstone)?;
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
            for record in &live {
                let line =
                    serde_json::to_string(*record).context("serialize history record")?;
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

/// Rename a speaker across one note: every `speaker_blocks` entry labeled
/// `from` takes `to`, and legacy chunk/session-speaker labels follow so old
/// chunk-tier notes stay self-consistent. Returns how many entries changed.
pub fn relabel_speaker_blocks(record: &mut HistoryRecord, from: &str, to: &str) -> usize {
    let mut changed = 0;
    for label in record
        .speaker_blocks
        .iter_mut()
        .map(|b| &mut b.label)
        .chain(record.speaker_chunks.iter_mut().map(|c| &mut c.label))
        .chain(record.session_speakers.iter_mut().map(|s| &mut s.label))
    {
        if label == from {
            *label = to.to_string();
            changed += 1;
        }
    }
    changed
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::note_sidecar;
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

    fn block(label: &str, start: u64, end: u64, text: &str) -> crate::types::SpeakerBlock {
        crate::types::SpeakerBlock {
            label: label.into(),
            start_ms: Some(start),
            end_ms: Some(end),
            text: text.into(),
            chunk_id: None,
        }
    }

    #[test]
    fn relabel_speaker_blocks_renames_all_matches_and_legacy_labels() {
        let mut rec = record("hello");
        rec.speaker_blocks = vec![
            block("Speaker 1", 0, 1_000, "one"),
            block("Speaker 2", 1_000, 2_000, "two"),
            block("Speaker 1", 2_000, 3_000, "three"),
        ];
        rec.speaker_chunks = vec![crate::types::SpeakerChunk {
            id: "chunk-0001".into(),
            start_ms: 0,
            end_ms: 1_000,
            label: "Speaker 1".into(),
            corrections: Vec::new(),
        }];
        rec.session_speakers = vec![crate::types::SessionSpeaker {
            session_speaker_id: "speaker-1".into(),
            label: "Speaker 1".into(),
            start_ms: 0,
            end_ms: 1_000,
            duration_ms: 1_000,
        }];

        let changed = relabel_speaker_blocks(&mut rec, "Speaker 1", "Ben");

        assert_eq!(changed, 4);
        assert_eq!(rec.speaker_blocks[0].label, "Ben");
        assert_eq!(rec.speaker_blocks[1].label, "Speaker 2");
        assert_eq!(rec.speaker_blocks[2].label, "Ben");
        assert_eq!(rec.speaker_chunks[0].label, "Ben");
        assert_eq!(rec.session_speakers[0].label, "Ben");
    }

    #[test]
    fn relabel_speaker_blocks_returns_zero_when_nothing_matches() {
        let mut rec = record("hello");
        rec.speaker_blocks = vec![block("Speaker 1", 0, 1_000, "one")];
        assert_eq!(relabel_speaker_blocks(&mut rec, "Speaker 9", "Ben"), 0);
        assert_eq!(rec.speaker_blocks[0].label, "Speaker 1");
    }

    #[test]
    fn relabel_speaker_persists_across_reload() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut rec = record("hello");
        rec.speaker_blocks = vec![
            block("Speaker 1", 0, 1_000, "one"),
            block("Speaker 2", 1_000, 2_000, "two"),
        ];
        let id = svc.append(&folder, rec).expect("append");

        let updated = svc
            .relabel_speaker(&folder, &id, "Speaker 1", "Ben")
            .expect("relabel");
        assert_eq!(updated.speaker_blocks[0].label, "Ben");

        let fresh = HistoryService::new();
        let got = fresh.get(&folder, &id).unwrap().expect("present");
        assert_eq!(got.speaker_blocks[0].label, "Ben");
        assert_eq!(got.speaker_blocks[1].label, "Speaker 2");
    }

    #[test]
    fn relabel_speaker_errors_on_unknown_note_or_label() {
        let folder = temp_folder();
        let svc = HistoryService::new();
        let mut rec = record("hello");
        rec.speaker_blocks = vec![block("Speaker 1", 0, 1_000, "one")];
        let id = svc.append(&folder, rec).expect("append");

        assert!(svc.relabel_speaker(&folder, "missing", "Speaker 1", "Ben").is_err());
        assert!(svc.relabel_speaker(&folder, &id, "Speaker 9", "Ben").is_err());
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
                    start_ms: 0,
                    end_ms: 2_000,
                    duration_ms: 2_000,
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
