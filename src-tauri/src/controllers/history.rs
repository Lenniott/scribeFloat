use crate::services::config::ConfigService;
use crate::services::history::HistoryService;
use crate::services::output::{self, OutputService};
use crate::services::speaker_names::{is_reserved_speaker_label, SpeakerNameService};
use crate::types::{
    DashboardStats, HistoryItemSource, HistoryKind, HistoryListItem, HistoryRecord, RelabelScope,
    TagVocabularyEntry,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Read/orchestration controller for the History view. Owns no state machine: it merges the
/// canonical store (`HistoryService`) with legacy on-disk items (`OutputService`), renders/exports
/// markdown, and orchestrates deletes. All merge/dedupe logic lives here and nowhere else.
pub struct HistoryController {
    history: Arc<HistoryService>,
    output: Arc<OutputService>,
    config: Arc<ConfigService>,
    speaker_names: Arc<SpeakerNameService>,
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
        speaker_names: Arc<SpeakerNameService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            history,
            output,
            config,
            speaker_names,
        })
    }

    /// Create a new Written note record and persist it. Returns the new record id.
    pub fn create_written_note(&self) -> Result<String, String> {
        let title = format!("{}", chrono::Local::now().format("%H:%M %d/%m/%y"));
        let record = crate::types::HistoryRecord::from_written(title);
        let save_folder = self.config.get().save_folder;
        self.history
            .append(&save_folder, record)
            .map_err(|e| e.to_string())
    }

    /// Update the written content of an existing note. Content is raw markdown.
    pub fn save_written_content(&self, id: &str, content: &str) -> Result<(), String> {
        let save_folder = self.config.get().save_folder;
        self.history
            .update_written_content(&save_folder, id, content)
            .map_err(|e| e.to_string())
    }

    /// Update the title of a note record (log-structured update).
    pub fn save_title(&self, id: &str, title: &str) -> Result<(), String> {
        let save_folder = self.config.get().save_folder;
        self.history
            .update_title(&save_folder, id, title)
            .map_err(|e| e.to_string())
    }

    /// True when the note has no written body, no transcript segments, and an unmodified default title.
    pub fn is_empty(&self, id: &str) -> Result<bool, String> {
        let save_folder = self.config.get().save_folder;
        let record = self
            .history
            .get(&save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("note not found: {id}"))?;
        Ok(crate::services::note_sidecar::record_is_empty(&record))
    }

    /// True when tags, keywords, or layer_item_ids are set in the note sidecar.
    pub fn has_metadata(&self, id: &str) -> Result<bool, String> {
        let save_folder = self.config.get().save_folder;
        self.history
            .get(&save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("note not found: {id}"))?;
        let meta = crate::services::note_sidecar::read_meta(&save_folder, id).unwrap_or_default();
        Ok(crate::services::note_sidecar::meta_has_editor_metadata(
            &meta,
        ))
    }

    /// Persist tags to the note sidecar (used by metadata UI and tests).
    #[allow(dead_code)]
    pub fn update_tags(&self, id: &str, tags: Vec<String>) -> Result<(), String> {
        let save_folder = self.config.get().save_folder;
        self.history
            .get(&save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("note not found: {id}"))?;
        crate::services::note_sidecar::write_tags(&save_folder, id, tags).map_err(|e| e.to_string())
    }

    /// Rename a speaker in one note and remember the new name globally (unless
    /// it's an auto-assigned label like "Speaker 2" or "Other").
    ///
    /// `scope: All` renames every turn labeled `from_label` (requires it);
    /// `scope: One` renames only the turn at `block_index` (requires it),
    /// leaving other turns sharing that label untouched.
    pub fn relabel_speaker(
        &self,
        id: &str,
        to_label: &str,
        scope: RelabelScope,
        from_label: Option<&str>,
        block_index: Option<usize>,
    ) -> Result<HistoryRecord, String> {
        if is_legacy(id) {
            return Err("legacy items are read-only".to_string());
        }
        let to_label = to_label.trim();
        if to_label.is_empty() {
            return Err("speaker label cannot be empty".to_string());
        }
        if to_label.len() > 80 {
            return Err("speaker label is too long (max 80 characters)".to_string());
        }
        let save_folder = self.config.get().save_folder;
        let updated = match scope {
            RelabelScope::All => {
                let from_label = from_label.map(str::trim).unwrap_or_default();
                if from_label.is_empty() {
                    return Err("from_label is required to rename all turns".to_string());
                }
                if from_label == to_label {
                    return Err("new speaker label matches the current one".to_string());
                }
                self.history
                    .relabel_speaker(&save_folder, id, from_label, to_label)
                    .map_err(|e| e.to_string())?
            }
            RelabelScope::One => {
                let block_index = block_index
                    .ok_or_else(|| "block_index is required to rename a single turn".to_string())?;
                let current = self
                    .history
                    .get(&save_folder, id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("note `{id}` not found"))?;
                let current_label = current
                    .speaker_blocks
                    .get(block_index)
                    .ok_or_else(|| format!("block index {block_index} out of range"))?
                    .label
                    .as_str();
                if current_label == to_label {
                    return Err("new speaker label matches the current one".to_string());
                }
                self.history
                    .relabel_speaker_block(&save_folder, id, block_index, to_label)
                    .map_err(|e| e.to_string())?
            }
        };
        if !is_reserved_speaker_label(to_label) {
            // Global name save is a convenience; never fail the relabel over it.
            if let Err(e) = self.speaker_names.save(to_label, None) {
                tracing::warn!(error = %e, "failed to save relabeled speaker name globally");
            }
        }
        Ok(updated)
    }

    /// Attach a completed transcription pass onto an existing note.
    pub fn attach_transcript(
        &self,
        id: &str,
        attachment: crate::types::TranscriptAttachment,
    ) -> Result<(), String> {
        let cfg = self.config.get();
        self.history
            .attach_transcript(&cfg.save_folder, id, attachment)
            .map_err(|e| e.to_string())
    }

    /// Render transcript markdown as HTML for the note editor Transcript panel.
    ///
    /// Markdown is converted with a narrow option set, then scrubbed with ammonia
    /// so user-influenced text cannot inject scripts/handlers into `{@html}`.
    pub fn render_transcript_html(&self, id: &str) -> Result<String, String> {
        let markdown = self.render_markdown(id)?;
        Ok(markdown_to_safe_html(&markdown))
    }

    /// Unified, deduped, newest-first list: store records ∪ legacy `.md` not already in the store
    /// ∪ legacy dictate entries.
    pub fn list(&self) -> Result<Vec<HistoryListItem>, String> {
        let save_folder = self.config.get().save_folder;

        let store = self
            .history
            .list_summaries(&save_folder)
            .map_err(|e| e.to_string())?;
        tracing::debug!(
            save_folder = %save_folder,
            store_count = store.len(),
            scribe = store.iter().filter(|r| r.kind == HistoryKind::Scribe).count(),
            dictate = store.iter().filter(|r| r.kind == HistoryKind::Dictate).count(),
            transcribe = store.iter().filter(|r| r.kind == HistoryKind::Transcribe).count(),
            written = store.iter().filter(|r| r.kind == HistoryKind::Written).count(),
            "history_list store records"
        );
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
                    duration_secs: 0,
                    excerpt: None,
                    tags: Vec::new(),
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
                    duration_secs: 0,
                    excerpt: excerpt_from_legacy_text(&entry.text),
                    tags: Vec::new(),
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
        let record = self
            .history
            .get(&save_folder, id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "history record not found".to_string())?;
        Ok(record)
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
        let speaker_blocks = record.speaker_blocks;
        let input_label = cfg.input_label.clone();
        let output_label = cfg.output_label.clone();
        // Return only the paragraph-grouped body text — no YAML front matter, no headings.
        // The full .md format is reserved for export (history_export_markdown).
        if speaker_blocks.is_empty() {
            Ok(output::render_transcript_body(
                &record.segments,
                cfg.include_timestamps,
            ))
        } else {
            Ok(output::render_speaker_blocks_body(
                &speaker_blocks,
                &input_label,
                &output_label,
            ))
        }
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
        if matches!(record.kind, HistoryKind::Dictate | HistoryKind::Written) {
            return Err("dictate and written items are not exported to markdown".to_string());
        }

        // Reserve a non-colliding filename. transcript_path derives the model slug from a path's
        // file stem, so reconstruct the original `ggml-<model>.bin` shape.
        let save_folder = PathBuf::from(&cfg.save_folder);
        let model_path = PathBuf::from(format!("ggml-{}.bin", record.model));
        let dest = self
            .output
            .transcript_path(&save_folder, &model_path, &record.title);

        let speaker_blocks = record.speaker_blocks;
        let input_label = cfg.input_label.clone();
        let output_label = cfg.output_label.clone();

        if speaker_blocks.is_empty() {
            self.output.write_transcript(
                &record.segments,
                &record.notes,
                &record.title,
                &record.model,
                cfg.include_timestamps,
                &dest,
            )
        } else {
            self.output.write_speaker_blocks_transcript(
                &speaker_blocks,
                &record.title,
                &record.model,
                &input_label,
                &output_label,
                &dest,
            )
        }
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

    /// Dashboard home-screen metrics (store records only for week duration).
    pub fn dashboard_stats(&self) -> Result<DashboardStats, String> {
        let save_folder = self.config.get().save_folder;
        let store = self
            .history
            .list_summaries(&save_folder)
            .map_err(|e| e.to_string())?;
        let transcript_count = store.len();
        let recorded_this_week_secs = sum_duration_this_iso_week(&store);
        Ok(DashboardStats {
            transcript_count,
            recorded_this_week_secs,
            float_layers: None,
            drafts_to_review: None,
        })
    }

    /// Unique tag names and transcript counts for the Transcripts filter panel.
    pub fn tag_vocabulary(&self) -> Result<Vec<TagVocabularyEntry>, String> {
        let save_folder = self.config.get().save_folder;
        let store = self
            .history
            .list_summaries(&save_folder)
            .map_err(|e| e.to_string())?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for item in &store {
            for tag in &item.tags {
                let key = tag.trim();
                if key.is_empty() {
                    continue;
                }
                *counts.entry(key.to_string()).or_default() += 1;
            }
        }
        let mut out: Vec<TagVocabularyEntry> = counts
            .into_iter()
            .map(|(name, count)| TagVocabularyEntry { name, count })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

fn is_legacy(id: &str) -> bool {
    id.starts_with(LEGACY_MD_PREFIX) || id.starts_with(LEGACY_DICTATE_PREFIX)
}

/// Derive a short display title from free text (first few words).
fn short_title(text: &str) -> String {
    let joined = text
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = joined.trim_end_matches(|c: char| !c.is_alphanumeric());
    if trimmed.is_empty() {
        "Dictation".to_string()
    } else {
        trimmed.to_string()
    }
}

fn excerpt_from_legacy_text(text: &str) -> Option<String> {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return None;
    }
    const MAX: usize = 120;
    if flat.chars().count() <= MAX {
        Some(flat)
    } else {
        let truncated: String = flat.chars().take(MAX).collect();
        Some(format!("{truncated}…"))
    }
}

fn sum_duration_this_iso_week(store: &[HistoryListItem]) -> Option<i64> {
    use chrono::Datelike;
    let now = chrono::Utc::now();
    let this_week = now.iso_week();
    let this_year = now.year();
    let mut total_secs: i64 = 0;
    let mut any = false;
    for item in store {
        let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&item.created_at) else {
            continue;
        };
        let utc = dt.with_timezone(&chrono::Utc);
        if utc.iso_week() == this_week && utc.year() == this_year {
            total_secs += item.duration_secs.max(0);
            any = true;
        }
    }
    any.then_some(total_secs)
}

/// Canonicalize `path` and confirm it lives inside `save_folder` (the `read_transcript_at` idiom).
/// Returns the canonical path when safe, or `None` when it escapes or cannot be resolved.
fn within_save_folder(path: &str, save_folder: &str) -> Option<PathBuf> {
    let canonical = Path::new(path).canonicalize().ok()?;
    let base = Path::new(save_folder).canonicalize().ok()?;
    canonical.starts_with(&base).then_some(canonical)
}

/// Convert transcript markdown to HTML safe for webview `{@html}` injection.
///
/// Uses CommonMark defaults only (no `Options::all()` kitchen-sink). Raw HTML in the
/// source still parses (CommonMark), so the result is always run through ammonia with
/// a tight tag allowlist for note-body display.
fn markdown_to_safe_html(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let parser = Parser::new_ext(markdown, Options::empty());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Transcript panel needs paragraphs + light emphasis/structure from legacy .md.
    // No scripts, forms, iframes, event-handler attrs, or javascript: URLs.
    ammonia::Builder::default()
        .tags(
            [
                "p", "br", "strong", "em", "b", "i", "ul", "ol", "li", "h1", "h2", "h3", "a",
                "blockquote", "code", "pre", "hr",
            ]
            .into_iter()
            .collect(),
        )
        .link_rel(Some("noopener noreferrer"))
        .clean(&html_output)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Note, Segment};

    struct Fixture {
        save_folder: String,
        ctrl: Arc<HistoryController>,
        history: Arc<HistoryService>,
        speaker_names: Arc<SpeakerNameService>,
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
        let speaker_names = SpeakerNameService::load(root.join("speaker_names.json"));
        let ctrl = HistoryController::new(
            Arc::clone(&history),
            output,
            Arc::clone(&config),
            Arc::clone(&speaker_names),
        );
        Fixture {
            save_folder,
            ctrl,
            history,
            speaker_names,
        }
    }

    fn seg(text: &str) -> Vec<Segment> {
        vec![Segment {
            start_ms: 0,
            end_ms: 1_000,
            text: text.to_string(),
            source: None,
        }]
    }

    #[test]
    fn delete_removes_record_markdown_and_session_dir() {
        let f = fixture();
        // Kept audio session dir with a mic.wav, plus an exported markdown file.
        let session_dir = PathBuf::from(&f.save_folder).join("2026-06-01_10-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("mic.wav"), b"RIFFfake").unwrap();
        let md = PathBuf::from(&f.save_folder).join("Meeting_tiny.md");
        std::fs::write(
            &md,
            b"---\ntitle: 'Meeting'\nmodel: tiny\n---\n\n## Transcript\n\nhi\n",
        )
        .unwrap();

        let record = HistoryRecord::from_scribe(
            "Meeting".to_string(),
            "tiny".to_string(),
            seg("hello"),
            Vec::<Note>::new(),
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
        assert!(
            f.history.list(&f.save_folder).unwrap().is_empty(),
            "record tombstoned"
        );
        // Idempotent.
        f.ctrl.delete(&id).unwrap();
    }

    fn labeled_record(labels: &[&str]) -> HistoryRecord {
        let mut rec = HistoryRecord::from_dictate(&seg("note"), "note text", "tiny".to_string());
        rec.speaker_blocks = labels
            .iter()
            .enumerate()
            .map(|(i, label)| crate::types::SpeakerBlock {
                label: label.to_string(),
                start_ms: Some(i as u64 * 1_000),
                end_ms: Some((i as u64 + 1) * 1_000),
                text: format!("text {i}"),
                chunk_id: None,
            })
            .collect();
        rec
    }

    #[test]
    fn relabel_speaker_updates_note_and_saves_name_globally() {
        let f = fixture();
        let id = f
            .history
            .append(&f.save_folder, labeled_record(&["Speaker 1", "Speaker 2"]))
            .unwrap();

        let updated = f
            .ctrl
            .relabel_speaker(&id, "Ben", RelabelScope::All, Some("Speaker 1"), None)
            .unwrap();

        assert_eq!(updated.speaker_blocks[0].label, "Ben");
        assert_eq!(updated.speaker_blocks[1].label, "Speaker 2");
        let names: Vec<String> = f.speaker_names.list().into_iter().map(|n| n.name).collect();
        assert_eq!(names, vec!["Ben".to_string()]);
    }

    #[test]
    fn relabel_speaker_does_not_save_reserved_labels() {
        let f = fixture();
        let id = f
            .history
            .append(&f.save_folder, labeled_record(&["Ben"]))
            .unwrap();

        f.ctrl
            .relabel_speaker(&id, "Speaker 2", RelabelScope::All, Some("Ben"), None)
            .unwrap();

        assert!(f.speaker_names.list().is_empty());
    }

    #[test]
    fn relabel_speaker_rejects_bad_input() {
        let f = fixture();
        let id = f
            .history
            .append(&f.save_folder, labeled_record(&["Speaker 1"]))
            .unwrap();

        assert!(f
            .ctrl
            .relabel_speaker(&id, "  ", RelabelScope::All, Some("Speaker 1"), None)
            .is_err());
        assert!(f
            .ctrl
            .relabel_speaker(&id, "Speaker 1", RelabelScope::All, Some("Speaker 1"), None)
            .is_err());
        assert!(f
            .ctrl
            .relabel_speaker(
                &id,
                &"x".repeat(81),
                RelabelScope::All,
                Some("Speaker 1"),
                None
            )
            .is_err());
        assert!(f
            .ctrl
            .relabel_speaker(
                "md::legacy",
                "Ben",
                RelabelScope::All,
                Some("Speaker 1"),
                None
            )
            .is_err());
        // Nothing was saved globally by the failed attempts.
        assert!(f.speaker_names.list().is_empty());
    }

    #[test]
    fn relabel_speaker_one_renames_only_that_block() {
        let f = fixture();
        let id = f
            .history
            .append(
                &f.save_folder,
                labeled_record(&["Speaker 1", "Speaker 1", "Speaker 1"]),
            )
            .unwrap();

        let updated = f
            .ctrl
            .relabel_speaker(&id, "Ben", RelabelScope::One, None, Some(1))
            .unwrap();

        assert_eq!(updated.speaker_blocks[0].label, "Speaker 1");
        assert_eq!(updated.speaker_blocks[1].label, "Ben");
        assert_eq!(updated.speaker_blocks[2].label, "Speaker 1");
        let names: Vec<String> = f.speaker_names.list().into_iter().map(|n| n.name).collect();
        assert_eq!(names, vec!["Ben".to_string()]);
    }

    #[test]
    fn relabel_speaker_one_rejects_bad_input() {
        let f = fixture();
        let id = f
            .history
            .append(&f.save_folder, labeled_record(&["Speaker 1", "Speaker 2"]))
            .unwrap();

        // Missing block_index.
        assert!(f
            .ctrl
            .relabel_speaker(&id, "Ben", RelabelScope::One, None, None)
            .is_err());
        // Out-of-range block_index.
        assert!(f
            .ctrl
            .relabel_speaker(&id, "Ben", RelabelScope::One, None, Some(9))
            .is_err());
        // Same label as the block already carries.
        assert!(f
            .ctrl
            .relabel_speaker(&id, "Speaker 1", RelabelScope::One, None, Some(0))
            .is_err());
        // Missing from_label for scope All.
        assert!(f
            .ctrl
            .relabel_speaker(&id, "Ben", RelabelScope::All, None, None)
            .is_err());
        assert!(f
            .ctrl
            .relabel_speaker("md::legacy", "Ben", RelabelScope::One, None, Some(0))
            .is_err());
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
        std::fs::write(
            &md,
            b"---\ntitle: 'Dupe'\nmodel: tiny\n---\n\n## Transcript\n\nhi\n",
        )
        .unwrap();
        let record = HistoryRecord::from_scribe(
            "Dupe".to_string(),
            "tiny".to_string(),
            seg("hello"),
            Vec::<Note>::new(),
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
            false,
            false,
            None,
            None,
            None,
        );
        let id = f.history.append(&f.save_folder, rec).unwrap();
        let text = f.ctrl.render_markdown(&id).unwrap();
        assert!(!text.contains("---"), "must not contain YAML front matter");
        assert!(
            !text.contains("## Transcript"),
            "must not contain section heading"
        );
        assert!(
            text.contains("hello world"),
            "must contain the segment text"
        );
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
    fn render_transcript_html_contains_paragraph_tag() {
        let f = fixture();
        let rec = HistoryRecord::from_scribe(
            "Hello".to_string(),
            "tiny".to_string(),
            seg("Hello world"),
            Vec::<Note>::new(),
            false,
            false,
            None,
            None,
            None,
        );
        let id = f.history.append(&f.save_folder, rec).unwrap();
        let html = f.ctrl.render_transcript_html(&id).unwrap();
        assert!(html.contains("<p>"));
        assert!(html.contains("Hello world"));
    }

    #[test]
    fn markdown_to_safe_html_keeps_emphasis_and_paragraphs() {
        let html = markdown_to_safe_html("Hello **world**\n\nSecond line");
        assert!(html.contains("<p>"), "expected paragraph: {html}");
        assert!(
            html.contains("<strong>world</strong>") || html.contains("<b>world</b>"),
            "expected bold: {html}"
        );
        assert!(html.contains("Second line"), "expected body text: {html}");
    }

    #[test]
    fn markdown_to_safe_html_strips_script_and_event_handlers() {
        let html = markdown_to_safe_html(
            "Hi <script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n[x](javascript:alert(1))",
        );
        let lower = html.to_ascii_lowercase();
        assert!(
            !lower.contains("<script"),
            "script tag must not survive: {html}"
        );
        assert!(
            !lower.contains("onerror"),
            "event handler must not survive: {html}"
        );
        assert!(
            !lower.contains("javascript:"),
            "javascript: URL must not survive: {html}"
        );
        assert!(html.contains("Hi"), "safe text must remain: {html}");
    }

    #[test]
    fn render_transcript_html_sanitizes_segment_payload() {
        let f = fixture();
        let rec = HistoryRecord::from_scribe(
            "XSS".to_string(),
            "tiny".to_string(),
            seg(r#"Hello <img src=x onerror="alert(1)"> world"#),
            Vec::<Note>::new(),
            false,
            false,
            None,
            None,
            None,
        );
        let id = f.history.append(&f.save_folder, rec).unwrap();
        let html = f.ctrl.render_transcript_html(&id).unwrap();
        let lower = html.to_ascii_lowercase();
        assert!(!lower.contains("onerror"), "handler leaked: {html}");
        assert!(!lower.contains("<script"), "script leaked: {html}");
        assert!(html.contains("Hello"), "text missing: {html}");
        assert!(html.contains("world"), "text missing: {html}");
    }

    #[test]
    fn list_includes_legacy_dictate_entries() {
        let f = fixture();
        let json =
            r#"[{"id":"d1","timestamp":"2026-01-01T00:00:00Z","text":"legacy dictation here"}]"#;
        std::fs::write(
            PathBuf::from(&f.save_folder).join("dictate_history.json"),
            json,
        )
        .unwrap();

        let items = f.ctrl.list().unwrap();
        let legacy: Vec<_> = items
            .iter()
            .filter(|i| i.source == HistoryItemSource::LegacyDictate)
            .collect();
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].id, "dictate::d1");
        assert_eq!(legacy[0].word_count, 3);
    }

    #[test]
    fn is_empty_returns_true_for_fresh_note() {
        let f = fixture();
        let id = f.ctrl.create_written_note().unwrap();
        assert!(f.ctrl.is_empty(&id).unwrap());
    }

    #[test]
    fn is_empty_returns_false_after_content_added() {
        let f = fixture();
        let id = f.ctrl.create_written_note().unwrap();
        f.history
            .update_written_content(&f.save_folder, &id, "hello")
            .unwrap();
        assert!(!f.ctrl.is_empty(&id).unwrap());
    }

    #[test]
    fn has_metadata_returns_false_for_fresh_note() {
        let f = fixture();
        let id = f.ctrl.create_written_note().unwrap();
        assert!(!f.ctrl.has_metadata(&id).unwrap());
    }

    #[test]
    fn has_metadata_returns_true_after_tags_set() {
        let f = fixture();
        let id = f.ctrl.create_written_note().unwrap();
        f.ctrl.update_tags(&id, vec!["tag1".into()]).unwrap();
        assert!(f.ctrl.has_metadata(&id).unwrap());
    }
}
