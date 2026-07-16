//! Plain global speaker-name store: just text, no biometrics.
//!
//! Backs `speaker_names.json` in app data. Names exist so transcript speaker
//! labels ("Speaker 1") can be renamed to people ("Ben") and reused across
//! notes. Uniqueness is by case-insensitive slug; renaming a stored name to a
//! slug that already exists is rejected rather than silently merged.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeakerName {
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Lowercase alphanumeric words joined by single dashes. Moved verbatim from
/// the voiceprint module (same slugs, so migrated names keep their identity).
pub fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            previous_dash = false;
        } else if !previous_dash && !out.is_empty() {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Labels the app assigns automatically; relabeling *to* one of these must not
/// pollute the saved-name store. Case-sensitive: these are canonical strings
/// the backend itself writes ("other" is a legitimate human name).
pub fn is_reserved_speaker_label(label: &str) -> bool {
    if matches!(
        label,
        "Other" | "You" | crate::types::CHANNEL_LABEL_IN | crate::types::CHANNEL_LABEL_OUT
    ) {
        return true;
    }
    // "Speaker N" with N a positive integer without leading zero.
    label
        .strip_prefix("Speaker ")
        .is_some_and(|n| !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit()))
}

fn validated(name: &str) -> Result<(String, String)> {
    let name = name.trim();
    let slug = slugify(name);
    if name.is_empty() || slug.is_empty() {
        return Err(anyhow!("speaker name must contain letters or digits"));
    }
    Ok((name.to_string(), slug))
}

/// Add `name`, or refresh casing/timestamp when its slug already exists.
pub fn upsert_name(list: &mut Vec<SpeakerName>, name: &str, now: &str) -> Result<SpeakerName> {
    let (name, slug) = validated(name)?;
    if let Some(existing) = list.iter_mut().find(|n| n.slug == slug) {
        existing.name = name;
        existing.updated_at = now.to_string();
        return Ok(existing.clone());
    }
    let entry = SpeakerName {
        name,
        slug,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    };
    list.push(entry.clone());
    Ok(entry)
}

/// Rename the entry at `slug`. Fails if `slug` is unknown or the new name's
/// slug collides with a different existing entry.
pub fn rename_name(
    list: &mut [SpeakerName],
    slug: &str,
    new_name: &str,
    now: &str,
) -> Result<SpeakerName> {
    let (new_name, new_slug) = validated(new_name)?;
    if new_slug != slug && list.iter().any(|n| n.slug == new_slug) {
        return Err(anyhow!("a speaker named \"{new_name}\" already exists"));
    }
    let entry = list
        .iter_mut()
        .find(|n| n.slug == slug)
        .ok_or_else(|| anyhow!("no speaker name with slug \"{slug}\""))?;
    entry.name = new_name;
    entry.slug = new_slug;
    entry.updated_at = now.to_string();
    Ok(entry.clone())
}

/// Remove the entry at `slug`; false when absent.
pub fn delete_name(list: &mut Vec<SpeakerName>, slug: &str) -> bool {
    let before = list.len();
    list.retain(|n| n.slug != slug);
    list.len() != before
}

pub struct SpeakerNameService {
    path: PathBuf,
    names: Mutex<Vec<SpeakerName>>,
}

impl SpeakerNameService {
    /// Missing or unreadable file → empty store (logged), never fatal.
    pub fn load(path: PathBuf) -> Arc<Self> {
        let names = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Vec<SpeakerName>>(&bytes).unwrap_or_else(|e| {
                tracing::warn!(error = %e, path = %path.display(), "speaker names file unreadable — starting empty");
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Arc::new(Self {
            path,
            names: Mutex::new(names),
        })
    }

    /// Sorted case-insensitively by name for stable display.
    pub fn list(&self) -> Vec<SpeakerName> {
        let mut names = self.names.lock().expect("speaker names lock").clone();
        names.sort_by_key(|n| n.name.to_lowercase());
        names
    }

    /// `previous_slug: None` → add/upsert; `Some` → rename that entry.
    pub fn save(&self, name: &str, previous_slug: Option<&str>) -> Result<SpeakerName> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut names = self.names.lock().expect("speaker names lock");
        let entry = match previous_slug {
            Some(slug) => rename_name(&mut names, slug, name, &now)?,
            None => upsert_name(&mut names, name, &now)?,
        };
        self.persist(&names)?;
        Ok(entry)
    }

    pub fn delete(&self, slug: &str) -> Result<bool> {
        let mut names = self.names.lock().expect("speaker names lock");
        let deleted = delete_name(&mut names, slug);
        if deleted {
            self.persist(&names)?;
        }
        Ok(deleted)
    }

    fn persist(&self, names: &[SpeakerName]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).context("create speaker names dir")?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(names).context("serialize speaker names")?;
        std::fs::write(&tmp, json).context("write speaker names tmp")?;
        std::fs::rename(&tmp, &self.path).context("commit speaker names")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-07-16T12:00:00Z";
    const LATER: &str = "2026-07-16T13:00:00Z";

    #[test]
    fn slugify_matches_voiceprint_behavior() {
        assert_eq!(slugify("Ben Mizrany"), "ben-mizrany");
        assert_eq!(slugify("  Émile!  "), "mile");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn reserved_labels_cover_auto_assigned_names_only() {
        for reserved in ["Other", "You", "In", "Out", "Speaker 1", "Speaker 4", "Speaker 10"] {
            assert!(is_reserved_speaker_label(reserved), "{reserved}");
        }
        for free in ["Ben", "other", "speaker 1", "Speaker 0", "Speaker 01", "Speaker", "Speaker one"] {
            assert!(!is_reserved_speaker_label(free), "{free}");
        }
    }

    #[test]
    fn upsert_adds_new_name_with_timestamps() {
        let mut list = Vec::new();
        let entry = upsert_name(&mut list, "  Ben  ", NOW).unwrap();
        assert_eq!(entry.name, "Ben");
        assert_eq!(entry.slug, "ben");
        assert_eq!(entry.created_at, NOW);
        assert_eq!(entry.updated_at, NOW);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn upsert_same_slug_updates_casing_and_keeps_created_at() {
        let mut list = Vec::new();
        upsert_name(&mut list, "ben", NOW).unwrap();
        let entry = upsert_name(&mut list, "BEN", LATER).unwrap();
        assert_eq!(entry.name, "BEN");
        assert_eq!(entry.created_at, NOW);
        assert_eq!(entry.updated_at, LATER);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn upsert_rejects_empty_and_symbol_only_names() {
        let mut list = Vec::new();
        assert!(upsert_name(&mut list, "   ", NOW).is_err());
        assert!(upsert_name(&mut list, "!!!", NOW).is_err());
        assert!(list.is_empty());
    }

    #[test]
    fn rename_updates_name_slug_and_timestamp() {
        let mut list = Vec::new();
        upsert_name(&mut list, "Ben", NOW).unwrap();
        let entry = rename_name(&mut list, "ben", "Benjamin", LATER).unwrap();
        assert_eq!(entry.name, "Benjamin");
        assert_eq!(entry.slug, "benjamin");
        assert_eq!(entry.created_at, NOW);
        assert_eq!(entry.updated_at, LATER);
    }

    #[test]
    fn rename_rejects_collision_with_other_entry() {
        let mut list = Vec::new();
        upsert_name(&mut list, "Ben", NOW).unwrap();
        upsert_name(&mut list, "Sarah", NOW).unwrap();
        assert!(rename_name(&mut list, "sarah", "BEN", LATER).is_err());
    }

    #[test]
    fn rename_to_same_slug_recases() {
        let mut list = Vec::new();
        upsert_name(&mut list, "ben", NOW).unwrap();
        let entry = rename_name(&mut list, "ben", "Ben", LATER).unwrap();
        assert_eq!(entry.name, "Ben");
        assert_eq!(entry.slug, "ben");
    }

    #[test]
    fn rename_unknown_slug_errors() {
        let mut list = Vec::new();
        assert!(rename_name(&mut list, "ghost", "Ben", NOW).is_err());
    }

    #[test]
    fn delete_removes_and_reports() {
        let mut list = Vec::new();
        upsert_name(&mut list, "Ben", NOW).unwrap();
        assert!(delete_name(&mut list, "ben"));
        assert!(!delete_name(&mut list, "ben"));
        assert!(list.is_empty());
    }

    #[test]
    fn service_load_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let svc = SpeakerNameService::load(dir.path().join("speaker_names.json"));
        assert!(svc.list().is_empty());
    }

    #[test]
    fn service_load_corrupt_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("speaker_names.json");
        std::fs::write(&path, b"{not json").unwrap();
        assert!(SpeakerNameService::load(path).list().is_empty());
    }

    #[test]
    fn service_save_persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("speaker_names.json");
        let svc = SpeakerNameService::load(path.clone());
        svc.save("Ben", None).unwrap();
        svc.save("sarah", None).unwrap();
        let reloaded = SpeakerNameService::load(path);
        let names: Vec<String> = reloaded.list().into_iter().map(|n| n.name).collect();
        assert_eq!(names, vec!["Ben".to_string(), "sarah".to_string()]);
    }

    #[test]
    fn service_list_sorts_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        let svc = SpeakerNameService::load(dir.path().join("names.json"));
        svc.save("zoe", None).unwrap();
        svc.save("Adam", None).unwrap();
        svc.save("ben", None).unwrap();
        let names: Vec<String> = svc.list().into_iter().map(|n| n.name).collect();
        assert_eq!(names, vec!["Adam".to_string(), "ben".to_string(), "zoe".to_string()]);
    }

    #[test]
    fn service_rename_and_delete_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("names.json");
        let svc = SpeakerNameService::load(path.clone());
        svc.save("Ben", None).unwrap();
        svc.save("Benjamin", Some("ben")).unwrap();
        assert!(svc.delete("missing").is_ok_and(|deleted| !deleted));
        let reloaded = SpeakerNameService::load(path.clone());
        assert_eq!(reloaded.list()[0].slug, "benjamin");
        reloaded.delete("benjamin").unwrap();
        assert!(SpeakerNameService::load(path).list().is_empty());
    }
}
