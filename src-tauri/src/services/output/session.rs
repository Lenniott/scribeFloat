use crate::types::{Note, SessionManifest};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub(super) struct SessionNotesPayload<'a> {
    pub(super) format_version: u8,
    pub(super) title: &'a str,
    pub(super) wav_file: &'a str,
    pub(super) notes: &'a [Note],
}

/// Create a timestamped session directory inside save_folder.
pub(super) fn make_session_dir(save_folder: &str) -> Result<PathBuf> {
    let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let dir = PathBuf::from(save_folder).join(&ts);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Write `[session_dir]/notes.json` capturing title and recorded notes for a WAV-only save.
pub(super) fn write_session_notes(
    session_dir: &Path,
    title: &str,
    wav_file_name: &str,
    notes: &[Note],
) -> Result<PathBuf> {
    let payload = SessionNotesPayload {
        format_version: 1,
        title,
        wav_file: wav_file_name,
        notes,
    };
    let dest = session_dir.join("notes.json");
    let json = serde_json::to_string_pretty(&payload).context("failed to serialize notes.json")?;
    std::fs::write(&dest, json).context("failed to write notes.json")?;
    Ok(dest)
}

/// Write or replace `{session_dir}/session.json` for Scribe lifecycle tracking.
pub(super) fn write_session_manifest(session_dir: &Path, manifest: &SessionManifest) -> Result<()> {
    let dest = session_dir.join("session.json");
    let json = serde_json::to_string_pretty(manifest).context("serialize session.json")?;
    std::fs::write(&dest, json).context("write session.json")?;
    Ok(())
}

/// After a successful Scribe transcription: drop `session.json` (and `notes.json`).
/// When `keep_wav` is false, delete staging WAVs and remove the session directory.
pub(super) fn finalize_scribe_session(session_dir: &Path, keep_wav: bool) -> Result<()> {
    if !session_dir.is_dir() {
        return Ok(());
    }
    let _ = std::fs::remove_file(session_dir.join("session.json"));
    let _ = std::fs::remove_file(session_dir.join("notes.json"));
    if keep_wav {
        return Ok(());
    }
    if let Ok(entries) = std::fs::read_dir(session_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    std::fs::remove_dir(session_dir).context("remove scribe session dir")?;
    Ok(())
}

/// Best-effort delete of all files in a Scribe staging dir, then the directory itself.
pub(super) fn remove_session_dir(dir: &Path) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    let _ = std::fs::remove_dir(dir);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Note, SessionManifest, SessionManifestState};

    fn temp_session_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("session-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn write_session_notes_roundtrip() {
        let dir = temp_session_dir();
        let notes = vec![Note {
            id: "n1".to_string(),
            text: "remember this".to_string(),
            recorded_at_ms: 2500,
        }];
        let dest =
            write_session_notes(&dir, "Meeting A", "mic.wav", &notes).expect("write_session_notes");
        assert_eq!(dest.file_name().unwrap(), "notes.json");
        let raw = std::fs::read_to_string(&dest).expect("read");
        assert!(raw.contains("Meeting A"));
        assert!(raw.contains("mic.wav"));
        assert!(raw.contains("remember this"));
    }

    #[test]
    fn write_session_manifest_roundtrip() {
        let dir = temp_session_dir();
        let manifest = SessionManifest {
            format_version: 1,
            state: SessionManifestState::Recording,
            started_at: "2026-05-28T12:00:00Z".to_string(),
            mic_wav: "mic.wav".to_string(),
            speaker_wavs: vec!["speaker_seg_0.wav".to_string()],
            transcript_path: None,
            title: None,
            speaker_cuts: vec![],
        };
        write_session_manifest(&dir, &manifest).expect("write");
        let raw = std::fs::read_to_string(dir.join("session.json")).expect("read");
        let parsed: SessionManifest = serde_json::from_str(&raw).expect("parse");
        assert_eq!(parsed.state, SessionManifestState::Recording);
        assert_eq!(parsed.speaker_wavs, vec!["speaker_seg_0.wav".to_string()]);
    }

    #[test]
    fn remove_session_dir_deletes_all_staging_files() {
        let dir = temp_session_dir();
        std::fs::write(dir.join("session.json"), b"{}").unwrap();
        std::fs::write(dir.join("mic.wav"), b"fake wav").unwrap();
        remove_session_dir(&dir);
        assert!(!dir.exists());
    }

    #[test]
    fn finalize_removes_dir_when_keep_wav_off() {
        let parent = temp_session_dir();
        let session_dir = parent.join("2026-05-28_13-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("session.json"), b"{}").unwrap();
        std::fs::write(session_dir.join("mic.wav"), b"fake wav").unwrap();
        finalize_scribe_session(&session_dir, false).expect("finalize");
        assert!(!session_dir.exists());
    }

    #[test]
    fn finalize_keeps_wavs_when_keep_wav_on() {
        let parent = temp_session_dir();
        let session_dir = parent.join("2026-05-28_14-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("session.json"), b"{}").unwrap();
        std::fs::write(session_dir.join("notes.json"), b"{}").unwrap();
        std::fs::write(session_dir.join("mic.wav"), b"fake wav").unwrap();
        finalize_scribe_session(&session_dir, true).expect("finalize");
        assert!(session_dir.is_dir());
        assert!(session_dir.join("mic.wav").is_file());
        assert!(!session_dir.join("session.json").exists());
        assert!(!session_dir.join("notes.json").exists());
    }
}
