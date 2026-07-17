//! One-time startup purge of legacy voiceprint biometric data.
//!
//! The voiceprint feature stored per-person voice embeddings under
//! `{data_dir}/voiceprints/*.json` plus transient enrollment clips. The feature
//! is gone; this migration preserves the only non-biometric part — profile
//! *names* — into the plain speaker-name store, then deletes the files and the
//! keychain encryption key. History embeddings need no code here: the fields no
//! longer exist on the record types, so the startup compaction rewrites
//! `history.jsonl` without them.
//!
//! Idempotent by construction: every step is a no-op once its input is gone.
//! Name import must fully succeed before the profiles dir is removed, so a
//! failed launch retries next time instead of losing names.

use crate::services::speaker_names::SpeakerNameService;
use std::path::Path;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PurgeReport {
    pub names_imported: usize,
    pub profiles_dir_removed: bool,
    pub clips_dir_removed: bool,
}

/// Filesystem side only; the caller deletes the keychain key when
/// `profiles_dir_removed` (keeps this unit-testable without touching the
/// real macOS Keychain).
pub fn purge_legacy_voice_data(data_dir: &Path, names: &SpeakerNameService) -> PurgeReport {
    let mut report = PurgeReport::default();

    let profiles_dir = data_dir.join("voiceprints");
    if profiles_dir.is_dir() {
        let mut all_imported = true;
        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                match profile_name(&path) {
                    Some(name) => match names.save(&name, None) {
                        Ok(_) => report.names_imported += 1,
                        Err(e) => {
                            tracing::warn!(error = %e, profile = %path.display(), "could not import voiceprint name — keeping profiles for retry");
                            all_imported = false;
                        }
                    },
                    None => {
                        tracing::warn!(profile = %path.display(), "unreadable voiceprint profile skipped during purge");
                    }
                }
            }
        }
        if all_imported {
            match std::fs::remove_dir_all(&profiles_dir) {
                Ok(()) => report.profiles_dir_removed = true,
                Err(e) => {
                    tracing::warn!(error = %e, "could not delete voiceprints dir — will retry next launch")
                }
            }
        }
    }

    let clips_dir = data_dir.join("voiceprint_clips");
    if clips_dir.is_dir() {
        match std::fs::remove_dir_all(&clips_dir) {
            Ok(()) => report.clips_dir_removed = true,
            Err(e) => {
                tracing::warn!(error = %e, "could not delete voiceprint clips dir — will retry next launch")
            }
        }
    }

    report
}

/// Lenient read: only the `name` field matters; anything else (embeddings,
/// evidence, corruption) is ignored.
fn profile_name(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let name = value.get("name")?.as_str()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn setup() -> (tempfile::TempDir, Arc<SpeakerNameService>) {
        let dir = tempfile::tempdir().unwrap();
        let names = SpeakerNameService::load(dir.path().join("speaker_names.json"));
        (dir, names)
    }

    fn write_profile(dir: &Path, file: &str, json: &str) {
        let profiles = dir.join("voiceprints");
        std::fs::create_dir_all(&profiles).unwrap();
        std::fs::write(profiles.join(file), json).unwrap();
    }

    #[test]
    fn imports_names_then_removes_profile_and_clip_dirs() {
        let (dir, names) = setup();
        write_profile(
            dir.path(),
            "ben.json",
            r#"{"name":"Ben","slug":"ben","embedding":[0.1,0.2],"sample_count":3}"#,
        );
        write_profile(
            dir.path(),
            "sarah.json",
            r#"{"name":"Sarah","slug":"sarah","embedding":[0.3],"sample_count":1}"#,
        );
        std::fs::create_dir_all(dir.path().join("voiceprint_clips")).unwrap();

        let report = purge_legacy_voice_data(dir.path(), &names);

        assert_eq!(report.names_imported, 2);
        assert!(report.profiles_dir_removed);
        assert!(report.clips_dir_removed);
        assert!(!dir.path().join("voiceprints").exists());
        assert!(!dir.path().join("voiceprint_clips").exists());
        let imported: Vec<String> = names.list().into_iter().map(|n| n.name).collect();
        assert_eq!(imported, vec!["Ben".to_string(), "Sarah".to_string()]);
    }

    #[test]
    fn corrupt_profile_is_skipped_but_dir_still_removed() {
        let (dir, names) = setup();
        write_profile(dir.path(), "good.json", r#"{"name":"Ben","slug":"ben"}"#);
        write_profile(dir.path(), "bad.json", "{not json");
        write_profile(dir.path(), "nameless.json", r#"{"slug":"ghost"}"#);

        let report = purge_legacy_voice_data(dir.path(), &names);

        assert_eq!(report.names_imported, 1);
        assert!(report.profiles_dir_removed);
        assert_eq!(names.list().len(), 1);
    }

    #[test]
    fn nothing_to_do_when_dirs_absent() {
        let (dir, names) = setup();
        let report = purge_legacy_voice_data(dir.path(), &names);
        assert_eq!(report, PurgeReport::default());
        assert!(names.list().is_empty());
    }

    #[test]
    fn second_run_is_a_clean_noop() {
        let (dir, names) = setup();
        write_profile(dir.path(), "ben.json", r#"{"name":"Ben","slug":"ben"}"#);
        purge_legacy_voice_data(dir.path(), &names);

        let second = purge_legacy_voice_data(dir.path(), &names);

        assert_eq!(second, PurgeReport::default());
        assert_eq!(names.list().len(), 1);
    }

    #[test]
    fn existing_name_with_same_slug_is_upserted_not_duplicated() {
        let (dir, names) = setup();
        names.save("ben", None).unwrap();
        write_profile(dir.path(), "ben.json", r#"{"name":"Ben","slug":"ben"}"#);

        let report = purge_legacy_voice_data(dir.path(), &names);

        assert_eq!(report.names_imported, 1);
        let list = names.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Ben");
    }
}
