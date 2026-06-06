use crate::services::config::ConfigService;
use crate::services::history::HistoryService;
use crate::services::output::{self, OutputService};
use crate::types::{
    HistoryItemSource, HistoryKind, HistoryListItem, HistoryRecord,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Read/orchestration controller for the History view. Owns no state machine: it merges the
/// canonical store (`HistoryService`) with legacy on-disk items (`OutputService`), renders/exports
/// markdown, and orchestrates deletes. All merge/dedupe logic lives here and nowhere else.
pub struct HistoryController {
    history: Arc<HistoryService>,
    output: Arc<OutputService>,
    config: Arc<ConfigService>,
}

/// Prefix marking a legacy on-disk `.md` item id (read-only).
const LEGACY_MD_PREFIX: &str = "md::";
/// Prefix marking a legacy `dictate_history.json` entry id (read-only).
const LEGACY_DICTATE_PREFIX: &str = "dictate::";

impl HistoryController {
    pub fn new(
        history: Arc<HistoryService>,
        output: Arc<OutputService>,
        config: Arc<ConfigService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            history,
            output,
            config,
        })
    }

    /// Unified, deduped, newest-first list: store records ∪ legacy `.md` not already in the store
    /// ∪ legacy dictate entries.
    pub fn list(&self) -> Result<Vec<HistoryListItem>, String> {
        let save_folder = self.config.get().save_folder;

        let store = self
            .history
            .list_summaries(&save_folder)
            .map_err(|e| e.to_string())?;
        let known_markdown: HashSet<String> = store
            .iter()
            .filter_map(|r| r.markdown_path.clone())
            .collect();

        let mut items: Vec<HistoryListItem> = store;

        // Legacy `.md` files not already represented by a store record's markdown_path.
        if let Ok(legacy_md) = OutputService::list_transcript_metadata(&save_folder) {
            for entry in legacy_md {
                if known_markdown.contains(&entry.path) {
                    continue;
                }
                items.push(HistoryListItem {
                    id: format!("{LEGACY_MD_PREFIX}{}", entry.path),
                    kind: HistoryKind::Scribe,
                    created_at: entry.modified_at,
                    title: entry.title,
                    model: entry.model,
                    word_count: 0,
                    duration_ms: 0,
                    has_markdown: true,
                    markdown_path: Some(entry.path),
                    source: HistoryItemSource::LegacyMarkdown,
                });
            }
        }

        // Legacy dictate entries.
        if let Ok(legacy_dictate) = self.output.read_dictate_history(&save_folder) {
            for entry in legacy_dictate {
                items.push(HistoryListItem {
                    id: format!("{LEGACY_DICTATE_PREFIX}{}", entry.id),
                    kind: HistoryKind::Dictate,
                    created_at: entry.timestamp,
                    title: short_title(&entry.text),
                    model: String::new(),
                    word_count: entry.text.split_whitespace().count(),
                    duration_ms: 0,
                    has_markdown: false,
                    markdown_path: None,
                    source: HistoryItemSource::LegacyDictate,
                });
            }
        }

        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(items)
    }

    /// Full structured record for a store id. Legacy ids have no structured detail.
    pub fn get_detail(&self, id: &str) -> Result<HistoryRecord, String> {
        if is_legacy(id) {
            return Err("legacy items have no structured detail".to_string());
        }
        let save_folder = self.config.get().save_folder;
        self.history
            .get(&save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "history record not found".to_string())
    }

    /// Render markdown for preview (no file written). Works for store and both legacy sources.
    pub fn render_markdown(&self, id: &str) -> Result<String, String> {
        let cfg = self.config.get();
        if let Some(path) = id.strip_prefix(LEGACY_MD_PREFIX) {
            return self.read_legacy(path);
        }
        if let Some(legacy_id) = id.strip_prefix(LEGACY_DICTATE_PREFIX) {
            let entries = self
                .output
                .read_dictate_history(&cfg.save_folder)
                .map_err(|e| e.to_string())?;
            return entries
                .into_iter()
                .find(|e| e.id == legacy_id)
                .map(|e| e.text)
                .ok_or_else(|| "dictate entry not found".to_string());
        }
        let record = self
            .history
            .get(&cfg.save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "history record not found".to_string())?;
        // Return only the paragraph-grouped body text — no YAML front matter, no headings.
        // The full .md format is reserved for export (history_export_markdown).
        Ok(output::render_transcript_body(
            &record.segments,
            cfg.include_timestamps,
            &cfg.replacement_rules,
        ))
    }

    /// Export a store record to a `.md` file on demand and record the path. Dictate never exports.
    pub fn export_markdown(&self, id: &str) -> Result<String, String> {
        if is_legacy(id) {
            return Err("legacy items cannot be re-exported".to_string());
        }
        let cfg = self.config.get();
        let record = self
            .history
            .get(&cfg.save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "history record not found".to_string())?;
        if record.kind == HistoryKind::Dictate {
            return Err("dictate items are not exported to markdown".to_string());
        }

        // Reserve a non-colliding filename. transcript_path derives the model slug from a path's
        // file stem, so reconstruct the original `ggml-<model>.bin` shape.
        let save_folder = PathBuf::from(&cfg.save_folder);
        let model_path = PathBuf::from(format!("ggml-{}.bin", record.model));
        let dest = self
            .output
            .transcript_path(&save_folder, &model_path, &record.title);

        self.output
            .write_transcript(
                &record.segments,
                &record.notes,
                &record.title,
                &record.model,
                cfg.include_timestamps,
                &cfg.replacement_rules,
                &dest,
            )
            .map_err(|e| e.to_string())?;

        let dest_str = dest.to_string_lossy().into_owned();
        self.history
            .set_markdown_path(&cfg.save_folder, id, &dest_str)
            .map_err(|e| e.to_string())?;
        Ok(dest_str)
    }

    /// Fully remove a store record: tombstone it, then delete its exported `.md` and kept audio.
    /// Legacy items are read-only. File-removal failures are non-fatal (the record is already gone).
    pub fn delete(&self, id: &str) -> Result<(), String> {
        if is_legacy(id) {
            return Err("legacy items are read-only and cannot be deleted".to_string());
        }
        let save_folder = self.config.get().save_folder;
        let Some(record) = self
            .history
            .delete(&save_folder, id)
            .map_err(|e| e.to_string())?
        else {
            return Ok(()); // unknown / already deleted — idempotent
        };

        // Remove kept audio: prefer the whole session dir, fall back to a single wav.
        if let Some(dir) = record.session_dir.as_deref() {
            if let Some(safe) = within_save_folder(dir, &save_folder) {
                self.output.remove_session_dir(&safe);
            }
        } else if let Some(audio) = record.audio_path.as_deref() {
            if let Some(safe) = within_save_folder(audio, &save_folder) {
                if let Err(e) = self.output.delete_wav(&safe) {
                    tracing::warn!(path = %safe.display(), error = %e, "failed to delete audio file");
                }
            }
        }

        // Remove the exported markdown.
        if let Some(md) = record.markdown_path.as_deref() {
            if let Some(safe) = within_save_folder(md, &save_folder) {
                if let Err(e) = self.output.delete_file(&safe) {
                    tracing::warn!(path = %safe.display(), error = %e, "failed to delete markdown file");
                }
            }
        }
        Ok(())
    }

    /// Read a legacy `.md` file, rejecting any path outside the configured save folder.
    pub fn read_legacy(&self, path: &str) -> Result<String, String> {
        let save_folder = self.config.get().save_folder;
        let canonical = within_save_folder(path, &save_folder)
            .ok_or_else(|| "path is outside the configured save folder".to_string())?;
        self.output.read_transcript(&canonical)
    }
}

fn is_legacy(id: &str) -> bool {
    id.starts_with(LEGACY_MD_PREFIX) || id.starts_with(LEGACY_DICTATE_PREFIX)
}

/// Derive a short display title from free text (first few words).
fn short_title(text: &str) -> String {
    let joined = text.split_whitespace().take(8).collect::<Vec<_>>().join(" ");
    let trimmed = joined.trim_end_matches(|c: char| !c.is_alphanumeric());
    if trimmed.is_empty() {
        "Dictation".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Canonicalize `path` and confirm it lives inside `save_folder` (the `read_transcript_at` idiom).
/// Returns the canonical path when safe, or `None` when it escapes or cannot be resolved.
fn within_save_folder(path: &str, save_folder: &str) -> Option<PathBuf> {
    let canonical = Path::new(path).canonicalize().ok()?;
    let base = Path::new(save_folder).canonicalize().ok()?;
    canonical.starts_with(&base).then_some(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Note, Segment};

    struct Fixture {
        save_folder: String,
        ctrl: Arc<HistoryController>,
        history: Arc<HistoryService>,
    }

    fn fixture() -> Fixture {
        let root =
            std::env::temp_dir().join(format!("scribefloat-histctrl-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let save_folder = std::fs::canonicalize(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let config = ConfigService::load(root.join("config.json")).unwrap();
        let sf = save_folder.clone();
        config.update(|c| c.save_folder = sf).unwrap();

        let history = HistoryService::new();
        let output = OutputService::new();
        let ctrl = HistoryController::new(Arc::clone(&history), output, Arc::clone(&config));
        Fixture { save_folder, ctrl, history }
    }

    fn seg(text: &str) -> Vec<Segment> {
        vec![Segment { start_ms: 0, end_ms: 1_000, text: text.to_string() }]
    }

    #[test]
    fn delete_removes_record_markdown_and_session_dir() {
        let f = fixture();
        // Kept audio session dir with a mic.wav, plus an exported markdown file.
        let session_dir = PathBuf::from(&f.save_folder).join("2026-06-01_10-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("mic.wav"), b"RIFFfake").unwrap();
        let md = PathBuf::from(&f.save_folder).join("Meeting_tiny.md");
        std::fs::write(&md, b"---\ntitle: 'Meeting'\nmodel: tiny\n---\n\n## Transcript\n\nhi\n").unwrap();

        let record = HistoryRecord::from_scribe(
            "Meeting".to_string(),
            "tiny".to_string(),
            seg("in: hello"),
            Vec::<Note>::new(),
            &[],
            true,
            true,
            Some(session_dir.to_string_lossy().into_owned()),
            Some(session_dir.join("mic.wav").to_string_lossy().into_owned()),
            Some(md.to_string_lossy().into_owned()),
        );
        let id = f.history.append(&f.save_folder, record).unwrap();

        f.ctrl.delete(&id).unwrap();

        assert!(!md.exists(), "markdown should be deleted");
        assert!(!session_dir.exists(), "session dir should be removed");
        assert!(f.history.list(&f.save_folder).unwrap().is_empty(), "record tombstoned");
        // Idempotent.
        f.ctrl.delete(&id).unwrap();
    }

    #[test]
    fn delete_without_artifacts_succeeds() {
        let f = fixture();
        let record = HistoryRecord::from_dictate(&seg("note"), "note text", "tiny".to_string());
        let id = f.history.append(&f.save_folder, record).unwrap();
        f.ctrl.delete(&id).unwrap();
        assert!(f.history.list(&f.save_folder).unwrap().is_empty());
    }

    #[test]
    fn legacy_ids_are_read_only() {
        let f = fixture();
        assert!(f.ctrl.delete("md::/x/y.md").is_err());
        assert!(f.ctrl.delete("dictate::abc").is_err());
        assert!(f.ctrl.export_markdown("md::/x/y.md").is_err());
        assert!(f.ctrl.get_detail("dictate::abc").is_err());
    }

    #[test]
    fn list_dedupes_legacy_md_matching_store_markdown_path() {
        let f = fixture();
        // A real .md on disk that the store also references via markdown_path.
        let md = PathBuf::from(&f.save_folder).join("Dupe_tiny.md");
        std::fs::write(&md, b"---\ntitle: 'Dupe'\nmodel: tiny\n---\n\n## Transcript\n\nhi\n").unwrap();
        let record = HistoryRecord::from_scribe(
            "Dupe".to_string(),
            "tiny".to_string(),
            seg("hello"),
            Vec::<Note>::new(),
            &[],
            false,
            false,
            None,
            None,
            Some(md.to_string_lossy().into_owned()),
        );
        f.history.append(&f.save_folder, record).unwrap();

        let items = f.ctrl.list().unwrap();
        // Exactly one item for that markdown — the store record, not a duplicate legacy entry.
        let dupes: Vec<_> = items.iter().filter(|i| i.title == "Dupe").collect();
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].source, HistoryItemSource::Store);
    }

    #[test]
    fn render_markdown_returns_body_only_no_front_matter() {
        let f = fixture();
        let rec = HistoryRecord::from_scribe(
            "My Meeting".to_string(),
            "tiny".to_string(),
            seg("hello world"),
            vec![],
            &[],
            false,
            false,
            None,
            None,
            None,
        );
        let id = f.history.append(&f.save_folder, rec).unwrap();
        let text = f.ctrl.render_markdown(&id).unwrap();
        assert!(!text.contains("---"), "must not contain YAML front matter");
        assert!(!text.contains("## Transcript"), "must not contain section heading");
        assert!(text.contains("hello world"), "must contain the segment text");
    }

    #[test]
    fn render_markdown_dictate_returns_plain_text() {
        let f = fixture();
        let segs = seg("quick brown fox");
        let rec = HistoryRecord::from_dictate(&segs, "quick brown fox", "tiny".to_string());
        let id = f.history.append(&f.save_folder, rec).unwrap();
        let text = f.ctrl.render_markdown(&id).unwrap();
        assert!(!text.contains("---"));
        assert!(text.contains("quick brown fox"));
    }

    #[test]
    fn list_includes_legacy_dictate_entries() {
        let f = fixture();
        let json = r#"[{"id":"d1","timestamp":"2026-01-01T00:00:00Z","text":"legacy dictation here"}]"#;
        std::fs::write(PathBuf::from(&f.save_folder).join("dictate_history.json"), json).unwrap();

        let items = f.ctrl.list().unwrap();
        let legacy: Vec<_> = items
            .iter()
            .filter(|i| i.source == HistoryItemSource::LegacyDictate)
            .collect();
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].id, "dictate::d1");
        assert_eq!(legacy[0].word_count, 3);
    }
}
