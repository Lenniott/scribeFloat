//! Editor-owned note fields on disk — overwritten in place, not appended to `history.jsonl`.
//!
//! Layout: `{save_folder}/.notes/{id}/written.md` + `meta.json` (title; tags/keywords in 0047).
//! Interim until story 0050 per-note folders replace `.notes/` with ADR-0007 folder names.

use crate::types::HistoryRecord;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sidecar metadata for editor-driven fields (title now; tags/keywords in 0047).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoteSidecarMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_item_ids: Vec<String>,
}

pub fn note_dir(save_folder: &str, id: &str) -> PathBuf {
    PathBuf::from(save_folder).join(".notes").join(id)
}

pub fn written_path(save_folder: &str, id: &str) -> PathBuf {
    note_dir(save_folder, id).join("written.md")
}

pub fn meta_path(save_folder: &str, id: &str) -> PathBuf {
    note_dir(save_folder, id).join("meta.json")
}

fn ensure_dir(save_folder: &str, id: &str) -> Result<()> {
    std::fs::create_dir_all(note_dir(save_folder, id))
        .with_context(|| format!("create note sidecar directory for {id}"))?;
    Ok(())
}

pub fn read_written(save_folder: &str, id: &str) -> Option<String> {
    std::fs::read_to_string(written_path(save_folder, id)).ok()
}

pub fn write_written(save_folder: &str, id: &str, content: &str) -> Result<()> {
    ensure_dir(save_folder, id)?;
    std::fs::write(written_path(save_folder, id), content.as_bytes())
        .context("write written.md sidecar")?;
    Ok(())
}

pub fn read_meta(save_folder: &str, id: &str) -> Option<NoteSidecarMeta> {
    let raw = std::fs::read_to_string(meta_path(save_folder, id)).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn write_meta_title(save_folder: &str, id: &str, title: &str) -> Result<()> {
    ensure_dir(save_folder, id)?;
    let mut meta = read_meta(save_folder, id).unwrap_or_default();
    meta.title = Some(title.to_string());
    write_meta(save_folder, id, &meta)
}

fn write_meta(save_folder: &str, id: &str, meta: &NoteSidecarMeta) -> Result<()> {
    let json = serde_json::to_string(meta).context("serialize note meta sidecar")?;
    std::fs::write(meta_path(save_folder, id), json.as_bytes())
        .context("write meta.json sidecar")?;
    Ok(())
}

/// Persist tags (and any existing meta fields) without touching jsonl.
#[allow(dead_code)]
pub fn write_tags(save_folder: &str, id: &str, tags: Vec<String>) -> Result<()> {
    ensure_dir(save_folder, id)?;
    let mut meta = read_meta(save_folder, id).unwrap_or_default();
    meta.tags = tags;
    write_meta(save_folder, id, &meta)
}

pub fn meta_has_editor_metadata(meta: &NoteSidecarMeta) -> bool {
    !meta.tags.is_empty() || !meta.keywords.is_empty() || !meta.layer_item_ids.is_empty()
}

/// True when title still matches the default generated at note creation time.
pub fn is_default_title(record: &HistoryRecord) -> bool {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&record.created_at) else {
        return false;
    };
    let expected = dt
        .with_timezone(&chrono::Local)
        .format("%H:%M %d/%m/%y")
        .to_string();
    record.title == expected
}

/// Empty note: no written body, no transcript segments, default title unchanged.
pub fn record_is_empty(record: &HistoryRecord) -> bool {
    let has_written = record
        .written_content
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    !has_written && record.segments.is_empty() && is_default_title(record)
}

/// Overlay sidecar files onto a record loaded from `history.jsonl`.
pub fn hydrate_record(save_folder: &str, record: &mut HistoryRecord) {
    if let Some(content) = read_written(save_folder, &record.id) {
        record.written_content = Some(content.clone());
        record.word_count = content.split_whitespace().count();
    }
    if let Some(meta) = read_meta(save_folder, &record.id) {
        if let Some(title) = meta.title.filter(|t| !t.is_empty()) {
            record.title = title;
        }
    }
}

pub fn remove_note_dir(save_folder: &str, id: &str) {
    let _ = std::fs::remove_dir_all(note_dir(save_folder, id));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_folder() -> String {
        let dir =
            std::env::temp_dir().join(format!("scribefloat-sidecar-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn write_meta_merges_existing_fields() {
        let folder = temp_folder();
        let id = "abc";
        write_meta_title(&folder, id, "first").unwrap();
        let mut meta = read_meta(&folder, id).unwrap();
        meta.title = Some("second".to_string());
        std::fs::write(
            meta_path(&folder, id),
            serde_json::to_string(&meta).unwrap(),
        )
        .unwrap();
        write_meta_title(&folder, id, "third").unwrap();
        assert_eq!(read_meta(&folder, id).unwrap().title.as_deref(), Some("third"));
    }

    #[test]
    fn write_tags_preserves_title() {
        let folder = temp_folder();
        let id = "tagged";
        write_meta_title(&folder, id, "My title").unwrap();
        write_tags(&folder, id, vec!["tag1".into()]).unwrap();
        let meta = read_meta(&folder, id).unwrap();
        assert_eq!(meta.title.as_deref(), Some("My title"));
        assert_eq!(meta.tags, vec!["tag1"]);
        assert!(meta_has_editor_metadata(&meta));
    }

    #[test]
    fn record_is_empty_respects_written_and_title() {
        let mut rec = crate::types::HistoryRecord::from_written("18:02 21/06/26".into());
        rec.created_at = chrono::Utc::now().to_rfc3339();
        rec.title = chrono::Local::now().format("%H:%M %d/%m/%y").to_string();
        assert!(record_is_empty(&rec));
        rec.written_content = Some("hi".into());
        assert!(!record_is_empty(&rec));
    }
}
