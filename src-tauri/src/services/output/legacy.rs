use crate::types::{DictateHistoryEntry, RecoverySessionInfo, ScribeTranscriptEntry};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::wav::repair_wav_header_from_file_size;

/// Max bytes read when listing legacy `.md` metadata (front matter only).
const TRANSCRIPT_METADATA_READ_CAP: usize = 4096;

fn read_file_prefix(path: &Path, cap: usize) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; cap];
    let n = file.read(&mut buf).ok()?;
    buf.truncate(n);
    String::from_utf8(buf).ok()
}

fn parse_front_matter_field(content: &str, key: &str) -> Option<String> {
    let after_open = content.strip_prefix("---")?;
    let close = after_open.find("\n---")?;
    let front = &after_open[..close];
    for line in front.lines() {
        if let Some(after_key) = line.strip_prefix(key) {
            if let Some(rest) = after_key.strip_prefix(':') {
                let value = rest.trim().trim_matches('\'').trim_matches('"');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Delete a single file. Silent no-op if it no longer exists.
pub(super) fn delete_file(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Read a transcript file.
pub(super) fn read_transcript(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("failed to read transcript: {e}"))
}

/// Read all entries from `{save_folder}/dictate_history.json` (newest-first).
pub(super) fn read_dictate_history(save_folder: &str) -> Result<Vec<DictateHistoryEntry>> {
    let path = PathBuf::from(save_folder).join("dictate_history.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path).context("read dictate_history.json")?;
    serde_json::from_str(&raw).context("parse dictate_history.json")
}

/// Scan `save_folder` root for `*.md` files and return their metadata sorted newest-first.
pub(super) fn list_transcript_metadata(
    save_folder: &str,
) -> Result<Vec<ScribeTranscriptEntry>> {
    let dir = PathBuf::from(save_folder);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<ScribeTranscriptEntry> = std::fs::read_dir(&dir)
        .context("read save folder")?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|x| x.to_str()) == Some("md") && e.path().is_file()
        })
        .filter_map(|e| {
            let path = e.path();
            let prefix =
                read_file_prefix(&path, TRANSCRIPT_METADATA_READ_CAP).unwrap_or_default();
            let fallback_title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let title = parse_front_matter_field(&prefix, "title").unwrap_or(fallback_title);
            let model = parse_front_matter_field(&prefix, "model").unwrap_or_default();
            let mtime = std::fs::metadata(&path).ok()?.modified().ok()?;
            let modified_at: chrono::DateTime<chrono::Utc> = mtime.into();
            Some(ScribeTranscriptEntry {
                path: path.to_string_lossy().into_owned(),
                title,
                model,
                modified_at: modified_at.to_rfc3339(),
            })
        })
        .collect();

    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(entries)
}

/// Move a failed dictate capture into `{save_folder}/dictate_failures/{timestamp}.wav`.
pub(super) fn salvage_dictate_wav(save_folder: &str, source_wav: &Path) -> Result<PathBuf> {
    let dest_dir = PathBuf::from(save_folder).join("dictate_failures");
    std::fs::create_dir_all(&dest_dir).context("create dictate_failures dir")?;
    let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let dest = dest_dir.join(format!("{ts}.wav"));
    std::fs::rename(source_wav, &dest)
        .or_else(|_| -> Result<()> {
            std::fs::copy(source_wav, &dest).context("copy dictate failure wav")?;
            std::fs::remove_file(source_wav).context("remove dictate temp wav")?;
            Ok(())
        })
        .context("salvage dictate wav")?;
    Ok(dest)
}

/// Scan `save_folder` for Scribe sessions that did not reach `complete` and repair WAV headers.
pub(super) fn scan_incomplete_scribe_sessions(
    save_folder: &str,
) -> Result<Vec<RecoverySessionInfo>> {
    use crate::types::{SessionManifest, SessionManifestState};

    let root = PathBuf::from(save_folder);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&root).context("read save folder")? {
        let entry = entry.context("read save folder entry")?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let session_dir = entry.path();
        let manifest_path = session_dir.join("session.json");
        let mic_path = session_dir.join("mic.wav");
        if !manifest_path.is_file() {
            continue;
        }
        if !mic_path.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&manifest_path).context("read session.json")?;
        let manifest: SessionManifest = serde_json::from_str(&raw).unwrap_or(SessionManifest {
            format_version: 1,
            state: SessionManifestState::Recording,
            started_at: String::new(),
            mic_wav: "mic.wav".to_string(),
            speaker_wavs: vec![],
            transcript_path: None,
            title: None,
            speaker_change_cuts: Vec::new(),
        });
        if matches!(manifest.state, SessionManifestState::Complete) {
            continue;
        }
        let state_label = format!("{:?}", manifest.state).to_lowercase();
        if crate::services::audio::read_wav_mono_f32(&mic_path).is_err() {
            let _ = repair_wav_header_from_file_size(&mic_path);
        }
        found.push(RecoverySessionInfo {
            session_dir: session_dir.to_string_lossy().into_owned(),
            mic_wav: mic_path.to_string_lossy().into_owned(),
            state: state_label,
        });
    }
    Ok(found)
}

/// Salvage orphaned dictate temp WAVs after a crash (checkpointed headers).
pub(super) fn scan_and_salvage_dictate_temp_wavs(
    temp_dir: &Path,
    save_folder: &str,
) -> Result<Vec<PathBuf>> {
    if !temp_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut salvaged = Vec::new();
    for entry in std::fs::read_dir(temp_dir).context("read dictate temp dir")? {
        let entry = entry.context("read dictate temp entry")?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("wav") {
            continue;
        }
        if crate::services::audio::read_wav_mono_f32(&path).is_err() {
            let _ = repair_wav_header_from_file_size(&path);
        }
        if crate::services::audio::read_wav_mono_f32(&path).is_ok() {
            match salvage_dictate_wav(save_folder, &path) {
                Ok(dest) => salvaged.push(dest),
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "failed to salvage dictate temp wav")
                }
            }
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(salvaged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("legacy-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_transcript_metadata_empty_folder() {
        let dir = temp_dir();
        let result =
            list_transcript_metadata(dir.to_str().unwrap()).expect("list transcripts");
        assert!(result.is_empty());
    }

    #[test]
    fn list_transcript_metadata_reads_title_from_prefix_only() {
        let dir = temp_dir();
        let mut content =
            String::from("---\ntitle: 'Huge Doc'\nmodel: small\n---\n\n## Transcript\n\n");
        content.push_str(&"x".repeat(50_000));
        std::fs::write(dir.join("huge.md"), content).unwrap();
        let entries =
            list_transcript_metadata(dir.to_str().unwrap()).expect("list metadata");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Huge Doc");
        assert_eq!(entries[0].model, "small");
    }

    #[test]
    fn list_transcript_metadata_reads_title_and_model() {
        let dir = temp_dir();
        let content = "---\ntitle: 'My Meeting'\nduration_seconds: 30.0\nword_count: 50\ntoken_estimate: 65\nmodel: tiny\n---\n\n## Transcript\n\nHello world.\n";
        std::fs::write(dir.join("my_meeting_tiny.md"), content).unwrap();
        let entries =
            list_transcript_metadata(dir.to_str().unwrap()).expect("list transcripts");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "My Meeting");
        assert_eq!(entries[0].model, "tiny");
    }

    #[test]
    fn list_transcript_metadata_sorted_desc() {
        let dir = temp_dir();
        let content = |title: &str| {
            format!("---\ntitle: '{title}'\nmodel: tiny\n---\n\n## Transcript\n\nText.\n")
        };
        std::fs::write(dir.join("a.md"), content("Alpha")).unwrap();
        std::fs::write(dir.join("b.md"), content("Beta")).unwrap();
        let entries =
            list_transcript_metadata(dir.to_str().unwrap()).expect("list transcripts");
        assert_eq!(entries.len(), 2);
        assert!(entries[0].modified_at >= entries[1].modified_at);
    }

    #[test]
    fn read_dictate_history_returns_empty_when_file_missing() {
        let dir = temp_dir();
        let entries = read_dictate_history(dir.to_str().unwrap()).expect("read");
        assert!(entries.is_empty());
    }

    #[test]
    fn read_dictate_history_parses_legacy_file() {
        let dir = temp_dir();
        let json = r#"[{"id":"a1","timestamp":"2026-01-01T00:00:00Z","text":"second"},
                       {"id":"a0","timestamp":"2025-12-31T00:00:00Z","text":"first"}]"#;
        std::fs::write(dir.join("dictate_history.json"), json).unwrap();
        let entries = read_dictate_history(dir.to_str().unwrap()).expect("read");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second");
        assert_eq!(entries[1].id, "a0");
    }
}
