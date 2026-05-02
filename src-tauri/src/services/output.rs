use crate::types::{DictateHistoryEntry, Note, Segment};
use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct OutputService;

#[derive(Debug, Serialize)]
struct SessionNotesPayload<'a> {
    format_version: u8,
    title: &'a str,
    wav_file: &'a str,
    notes: &'a [Note],
}

impl OutputService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    /// Create a timestamped session directory inside save_folder.
    pub fn make_session_dir(&self, save_folder: &str) -> Result<PathBuf> {
        let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let dir = PathBuf::from(save_folder).join(&ts);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Build the transcript file path using the recording title as the filename base.
    /// Spaces become underscores; chars forbidden on Windows/macOS become dashes.
    pub fn transcript_path(&self, session_dir: &Path, model_path: &Path, title: &str) -> PathBuf {
        let slug: String = title
            .chars()
            .map(|c| match c {
                ' ' => '_',
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                c => c,
            })
            .collect();
        let slug = if slug.is_empty() {
            chrono::Local::now()
                .format("%Y-%m-%d_%H-%M-%S")
                .to_string()
        } else {
            slug
        };
        let stem = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "model".to_string());
        session_dir.join(format!("{}_{}.md", slug, stem))
    }

    /// Write mono f32 PCM as a 16-bit WAV file.
    pub fn write_wav(&self, pcm: &[f32], sample_rate: u32, dest: &Path) -> Result<()> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(dest, spec).context("failed to create WAV writer")?;
        for &s in pcm {
            let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Render segments as markdown and write. Verifies file is non-empty before returning Ok.
    pub fn write_transcript(
        &self,
        segments: &[Segment],
        notes: &[Note],
        title: &str,
        model_name: &str,
        include_timestamps: bool,
        dest: &Path,
    ) -> Result<PathBuf> {
        let transcript_body = segments
            .iter()
            .map(|seg| {
                if include_timestamps {
                    format!("[{}] {}", format_ms(seg.start_ms), seg.text)
                } else {
                    seg.text.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let duration_seconds = segments
            .last()
            .map(|s| s.end_ms.max(0) as f64 / 1000.0)
            .unwrap_or(0.0);
        let word_count = transcript_body.split_whitespace().count();
        let token_estimate = ((word_count as f64) * 1.3).round() as usize;

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("title: '{}'\n", title.replace('\'', "’")));
        md.push_str(&format!("duration_seconds: {:.1}\n", duration_seconds));
        md.push_str(&format!("word_count: {word_count}\n"));
        md.push_str(&format!("token_estimate: {token_estimate}\n"));
        md.push_str(&format!("model: {model_name}\n"));
        md.push_str("---\n\n");
        md.push_str("## Transcript\n\n");
        md.push_str(&transcript_body);

        if !notes.is_empty() {
            md.push_str("\n\n## Notes\n");
            for (i, note) in notes.iter().enumerate() {
                md.push_str(&format!(
                    "[{}] ({}) {}\n",
                    i + 1,
                    format_ms(note.recorded_at_ms as i64),
                    note.text
                ));
            }
        }

        md.push('\n');
        std::fs::write(dest, &md).context("failed to write transcript")?;
        if std::fs::metadata(dest)?.len() == 0 {
            return Err(anyhow::anyhow!("transcript was written empty"));
        }
        Ok(dest.to_path_buf())
    }

    /// Delete a WAV file. Silent no-op if it no longer exists.
    pub fn delete_wav(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Write `[session_dir]/notes.json` capturing title and recorded notes for a WAV-only save.
    pub fn write_session_notes(
        &self,
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
        let json =
            serde_json::to_string_pretty(&payload).context("failed to serialize notes.json")?;
        std::fs::write(&dest, json).context("failed to write notes.json")?;
        Ok(dest)
    }

    /// Read a transcript file. Path validation (boundary check) is the caller's responsibility;
    /// the actual I/O lives here to keep all disk access inside OutputService.
    pub fn read_transcript(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| format!("failed to read transcript: {e}"))
    }

    /// Create the directory at `path` (and missing parents) and return its canonical form.
    /// Used when validating and persisting a new save-folder location.
    pub fn ensure_output_dir(&self, path: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("failed to create directory `{}`: {e}", path.display()))?;
        let canonical = std::fs::canonicalize(path)
            .map_err(|e| format!("failed to resolve path `{}`: {e}", path.display()))?;
        if !canonical.is_dir() {
            return Err(format!("`{}` is not a directory", canonical.display()));
        }
        Ok(canonical)
    }

    /// Open a file with the OS default handler (or a named app). Delegates to platform code
    /// so that controllers do not call the platform layer directly.
    pub fn open_file_for_user(&self, path: &str, app: Option<&str>) -> Result<(), String> {
        crate::platform::open_file(path, app)
    }

    /// Prepend a new entry to `{save_folder}/dictate_history.json`.
    /// Creates the file if it does not exist. The list is newest-first.
    pub fn write_dictate_history_entry(&self, save_folder: &str, text: &str) -> Result<()> {
        let path = PathBuf::from(save_folder).join("dictate_history.json");
        let mut entries: Vec<DictateHistoryEntry> = if path.exists() {
            let raw = std::fs::read_to_string(&path).context("read dictate_history.json")?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            Vec::new()
        };
        entries.insert(0, DictateHistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            text: text.to_string(),
        });
        let json = serde_json::to_string_pretty(&entries)
            .context("serialize dictate_history.json")?;
        std::fs::write(&path, json).context("write dictate_history.json")?;
        Ok(())
    }

    /// Read all entries from `{save_folder}/dictate_history.json` (newest-first).
    /// Returns an empty list if the file does not exist.
    pub fn read_dictate_history(&self, save_folder: &str) -> Result<Vec<DictateHistoryEntry>> {
        let path = PathBuf::from(save_folder).join("dictate_history.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&path).context("read dictate_history.json")?;
        serde_json::from_str(&raw).context("parse dictate_history.json")
    }

    /// Simulate Cmd/Ctrl+V into the currently focused application.
    /// Requires Accessibility permission on macOS. Caller must write text to clipboard first.
    pub fn paste_text(&self) -> Result<(), String> {
        crate::platform::paste_impl::paste_text()
    }

    /// Simulate pressing Enter in the currently focused application.
    pub fn send_enter(&self) -> Result<(), String> {
        crate::platform::paste_impl::send_enter()
    }

    /// Remove the session directory if it contains no files (i.e. recording was cancelled
    /// before any WAV was written). Silent no-op if the directory is non-empty or gone.
    pub fn delete_session_dir_if_empty(&self, dir: &Path) {
        if !dir.exists() {
            return;
        }
        let is_empty = std::fs::read_dir(dir)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

fn format_ms(ms: i64) -> String {
    let total = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Note;


    fn temp_file(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liscribe-output-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join(name)
    }

    #[test]
    fn transcript_renders_timestamps_when_enabled() {
        let svc = OutputService;
        let file = temp_file("with-timestamps.md");
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
        }];

        svc.write_transcript(&segments, &[], "Test", "tiny", true, &file)
            .expect("write transcript");

        let content = std::fs::read_to_string(&file).expect("read transcript");
        assert!(content.contains("[00:00:12] hello world"));
    }

    #[test]
    fn transcript_omits_timestamps_when_disabled() {
        let svc = OutputService;
        let file = temp_file("without-timestamps.md");
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
        }];

        svc.write_transcript(&segments, &[], "Test", "tiny", false, &file)
            .expect("write transcript");

        let content = std::fs::read_to_string(&file).expect("read transcript");
        assert!(content.contains("hello world"));
        assert!(!content.contains("[00:00:12]"));
    }

    #[test]
    fn session_notes_json_includes_title_wav_and_note_text() {
        let svc = OutputService;
        let dir = std::env::temp_dir().join(format!("liscribe-notes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let notes = vec![Note {
            id: "n1".to_string(),
            text: "remember this".to_string(),
            recorded_at_ms: 2500,
        }];
        let dest = svc
            .write_session_notes(&dir, "Meeting A", "mic.wav", &notes)
            .expect("write_session_notes");
        assert_eq!(dest.file_name().unwrap(), "notes.json");
        let raw = std::fs::read_to_string(&dest).expect("read");
        assert!(raw.contains("Meeting A"));
        assert!(raw.contains("mic.wav"));
        assert!(raw.contains("remember this"));
    }
}
