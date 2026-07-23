use crate::services::speaker_names::{SpeakerName, SpeakerNameService};
use std::sync::Arc;

/// Command orchestration for the plain speaker-name store (Settings → Voices
/// and the transcript relabel picker). Validation beyond emptiness lives in the
/// service's pure core; this layer only translates errors for IPC.
pub struct SpeakerNamesController {
    names: Arc<SpeakerNameService>,
}

impl SpeakerNamesController {
    pub fn new(names: Arc<SpeakerNameService>) -> Arc<Self> {
        Arc::new(Self { names })
    }

    pub fn list(&self) -> Vec<SpeakerName> {
        self.names.list()
    }

    pub fn save(&self, name: &str, previous_slug: Option<&str>) -> Result<SpeakerName, String> {
        let name = name.trim();
        if name.len() > 80 {
            return Err("speaker name is too long (max 80 characters)".to_string());
        }
        self.names
            .save(name, previous_slug.map(str::trim).filter(|s| !s.is_empty()))
            .map_err(|e| e.to_string())
    }

    pub fn delete(&self, slug: &str) -> Result<bool, String> {
        self.names.delete(slug.trim()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> (tempfile::TempDir, Arc<SpeakerNamesController>) {
        let dir = tempfile::tempdir().unwrap();
        let svc = SpeakerNameService::load(dir.path().join("speaker_names.json"));
        (dir, SpeakerNamesController::new(svc))
    }

    #[test]
    fn save_list_rename_delete_roundtrip() {
        let (_dir, ctrl) = ctrl();
        ctrl.save("Ben", None).unwrap();
        ctrl.save("Benjamin", Some("ben")).unwrap();
        assert_eq!(ctrl.list()[0].slug, "benjamin");
        assert!(ctrl.delete("benjamin").unwrap());
        assert!(ctrl.list().is_empty());
    }

    #[test]
    fn save_rejects_over_long_names() {
        let (_dir, ctrl) = ctrl();
        assert!(ctrl.save(&"x".repeat(81), None).is_err());
    }
}
