use crate::types::{
    DictateHistoryEntry, Note, RecoverySessionInfo, ReplacementRule, ReplacementRuleType,
    ReplacementScope, ScribeTranscriptEntry, Segment, SessionManifest, SessionManifestState,
    WordTransform,
};
use anyhow::{anyhow, Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use regex::Regex;
use serde::Serialize;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct OutputService;

/// Max bytes read when listing legacy `.md` metadata (front matter only).
const TRANSCRIPT_METADATA_READ_CAP: usize = 4096;

/// Read up to `cap` bytes from `path` for front-matter parsing (avoids loading full transcripts on list).
fn read_file_prefix(path: &Path, cap: usize) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; cap];
    let n = file.read(&mut buf).ok()?;
    buf.truncate(n);
    String::from_utf8(buf).ok()
}

/// Extract a scalar value from a YAML front matter block (the `---` delimited header).
/// Handles both quoted (`key: 'value'`) and unquoted (`key: value`) forms.
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

fn transcript_filename_base(model_path: &Path, title: &str) -> String {
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
    format!("{slug}_{stem}")
}

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

    /// Build a transcript path in the save folder root: `{save_folder}/{title}_{model}.md`.
    /// When that file already exists, appends `_1`, `_2`, … before `.md`. Spaces in the title
    /// become underscores; forbidden path chars become dashes.
    pub fn transcript_path(&self, save_folder: &Path, model_path: &Path, title: &str) -> PathBuf {
        let base = transcript_filename_base(model_path, title);
        let candidate = save_folder.join(format!("{base}.md"));
        if !candidate.exists() {
            return candidate;
        }
        for n in 1.. {
            let numbered = save_folder.join(format!("{base}_{n}.md"));
            if !numbered.exists() {
                return numbered;
            }
        }
        unreachable!("transcript_path suffix loop is bounded by filesystem")
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

    /// Join segments, clean Whisper artifacts, apply replacement rules, and return the final
    /// text ready for pasting. Scope applied: Dictate.
    pub fn format_dictate_text(
        &self,
        segments: &[Segment],
        rules: &[ReplacementRule],
        prefix: &str,
    ) -> String {
        let joined = segments
            .iter()
            .map(|s| cleanup_text(s.text.trim()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let deduped = dedup_repeated_block(&dedup_consecutive_phrases(&dedup_exact_halves(&joined)));
        apply_replacements(&deduped, rules, &ReplacementScope::Dictate, prefix)
    }

    /// Render segments as markdown and write. Verifies file is non-empty before returning Ok.
    /// This is now a thin wrapper around the pure [`render_transcript_markdown`] renderer so
    /// the exact same markdown is produced at capture time, on-demand export, and preview.
    #[allow(clippy::too_many_arguments)]
    pub fn write_transcript(
        &self,
        segments: &[Segment],
        notes: &[Note],
        title: &str,
        model_name: &str,
        include_timestamps: bool,
        rules: &[ReplacementRule],
        prefix: &str,
        dest: &Path,
    ) -> Result<PathBuf> {
        let md =
            render_transcript_markdown(segments, notes, title, model_name, include_timestamps, rules, prefix);
        std::fs::write(dest, &md).context("failed to write transcript")?;
        if std::fs::metadata(dest)?.len() == 0 {
            return Err(anyhow::anyhow!("transcript was written empty"));
        }
        Ok(dest.to_path_buf())
    }

    /// Delete a single file. Silent no-op if it no longer exists. The generic primitive
    /// used by the History delete path to remove an exported `.md`.
    pub fn delete_file(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Delete a WAV file. Silent no-op if it no longer exists. Delegates to [`delete_file`].
    pub fn delete_wav(&self, path: &Path) -> Result<()> {
        self.delete_file(path)
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

    /// Scan `save_folder` root for `*.md` files and return their metadata sorted newest-first.
    /// Reads only a bounded prefix of each file for YAML front matter (title, model).
    pub fn list_transcripts(&self, save_folder: &str) -> Result<Vec<ScribeTranscriptEntry>> {
        Self::list_transcript_metadata(save_folder)
    }

    /// Same as [`list_transcripts`](Self::list_transcripts) — bounded read for History list performance.
    pub fn list_transcript_metadata(save_folder: &str) -> Result<Vec<ScribeTranscriptEntry>> {
        let dir = PathBuf::from(save_folder);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries: Vec<ScribeTranscriptEntry> = std::fs::read_dir(&dir)
            .context("read save folder")?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("md")
                    && e.path().is_file()
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
                let title =
                    parse_front_matter_field(&prefix, "title").unwrap_or(fallback_title);
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

    /// Simulate Cmd/Ctrl+V into the currently focused application.
    /// Requires Accessibility permission on macOS. Caller must write text to clipboard first.
    pub fn paste_text(&self) -> Result<(), String> {
        crate::platform::paste_impl::paste_text()
    }

    /// Simulate pressing Enter in the currently focused application.
    pub fn send_enter(&self) -> Result<(), String> {
        crate::platform::paste_impl::send_enter()
    }

    /// Best-effort delete of all files in a Scribe staging dir, then the directory itself.
    pub fn remove_session_dir(&self, dir: &Path) {
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

    /// After a successful Scribe transcription: drop `session.json` (and `notes.json`).
    /// When `keep_wav` is false, delete staging WAVs and remove the session directory
    /// (the transcript `.md` lives at the save-folder root, not inside this folder).
    pub fn finalize_scribe_session(
        &self,
        session_dir: &Path,
        keep_wav: bool,
    ) -> Result<()> {
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

    /// Write or replace `{session_dir}/session.json` for Scribe lifecycle tracking.
    pub fn write_session_manifest(&self, session_dir: &Path, manifest: &SessionManifest) -> Result<()> {
        let dest = session_dir.join("session.json");
        let json = serde_json::to_string_pretty(manifest).context("serialize session.json")?;
        std::fs::write(&dest, json).context("write session.json")?;
        Ok(())
    }

    /// Move a failed dictate capture into `{save_folder}/dictate_failures/{timestamp}.wav`.
    pub fn salvage_dictate_wav(&self, save_folder: &str, source_wav: &Path) -> Result<PathBuf> {
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
    pub fn scan_incomplete_scribe_sessions(&self, save_folder: &str) -> Result<Vec<RecoverySessionInfo>> {
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
                // keep_wav archives: mic.wav without session.json — not incomplete.
                continue;
            }
            if !mic_path.is_file() {
                continue;
            }
            let raw = std::fs::read_to_string(&manifest_path).context("read session.json")?;
            let manifest: SessionManifest =
                serde_json::from_str(&raw).unwrap_or(SessionManifest {
                    format_version: 1,
                    state: SessionManifestState::Recording,
                    started_at: String::new(),
                    mic_wav: "mic.wav".to_string(),
                    speaker_wavs: vec![],
                    transcript_path: None,
                    title: None,
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
    pub fn scan_and_salvage_dictate_temp_wavs(
        &self,
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
                match self.salvage_dictate_wav(save_folder, &path) {
                    Ok(dest) => salvaged.push(dest),
                    Err(e) => tracing::warn!(path = %path.display(), error = %e, "failed to salvage dictate temp wav"),
                }
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        Ok(salvaged)
    }
}

// ── Transcript markdown rendering (pure — no I/O) ───────────────────────────────

/// Group segments into paragraphs and render the transcript body (after replacement rules).
/// Consecutive same-source segments separated by less than 8 s merge into one paragraph;
/// speaker sources (`in:` vs `out:`) never merge. This is the single body-rendering path
/// shared by the markdown writer, on-demand export/preview, and word counting.
pub fn render_transcript_body(
    segments: &[Segment],
    include_timestamps: bool,
    rules: &[ReplacementRule],
    prefix: &str,
) -> String {
    const MERGE_GAP_MS: i64 = 8_000;
    struct Group {
        start_ms: i64,
        end_ms: i64,
        parts: Vec<String>,
        source: &'static str, // "in", "out", or "" for single-source
    }
    let mut groups: Vec<Group> = Vec::new();
    for seg in segments {
        let clean = cleanup_text(seg.text.trim());
        if clean.is_empty() {
            continue;
        }
        let deduped = dedup_repeated_block(&dedup_consecutive_phrases(&clean));
        let seg_source = speaker_source_prefix(&deduped);
        let last = groups
            .last_mut()
            .filter(|g| seg.start_ms - g.end_ms < MERGE_GAP_MS && g.source == seg_source);
        match last {
            Some(g) => {
                g.end_ms = seg.end_ms;
                let body = deduped
                    .strip_prefix("in: ")
                    .or_else(|| deduped.strip_prefix("out: "))
                    .map(|s| s.to_string())
                    .unwrap_or(deduped);
                g.parts.push(body);
            }
            None => groups.push(Group {
                start_ms: seg.start_ms,
                end_ms: seg.end_ms,
                parts: vec![deduped],
                source: seg_source,
            }),
        }
    }
    let raw_body = {
        let mut out = String::new();
        for (i, g) in groups.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n");
            }
            let text = g.parts.join(" ");
            if include_timestamps {
                out.push_str(&format!("[{}] {}", format_ms(g.start_ms), text));
            } else {
                out.push_str(&text);
            }
        }
        out
    };
    apply_replacements(&raw_body, rules, &ReplacementScope::Transcripts, prefix)
}

/// Count words in the rendered transcript body, excluding timestamp labels. Shared by the
/// `HistoryRecord` builders and the markdown front matter so the store and the `.md` agree.
pub fn count_words(segments: &[Segment], rules: &[ReplacementRule], prefix: &str) -> usize {
    render_transcript_body(segments, false, rules, prefix)
        .split_whitespace()
        .count()
}

/// Render a complete transcript markdown document (YAML front matter + `## Transcript` +
/// optional `## Notes`, trailing newline). Pure: performs no file I/O.
pub fn render_transcript_markdown(
    segments: &[Segment],
    notes: &[Note],
    title: &str,
    model_name: &str,
    include_timestamps: bool,
    rules: &[ReplacementRule],
    prefix: &str,
) -> String {
    let transcript_body = render_transcript_body(segments, include_timestamps, rules, prefix);

    let duration_seconds = segments
        .last()
        .map(|s| s.end_ms.max(0) as f64 / 1000.0)
        .unwrap_or(0.0);
    let word_count = count_words(segments, rules, prefix);
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
    md
}


/// Write a placeholder 16-bit PCM WAV header for streaming capture.
pub fn write_streaming_wav_placeholder(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
) -> Result<()> {
    sync_wav_header(path, sample_rate, channels, bits_per_sample, 0)?;
    Ok(())
}

/// Patch RIFF/data chunk sizes for a 16-bit PCM WAV without finalizing the writer.
pub fn sync_wav_header(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    sample_count: u64,
) -> Result<()> {
    let block_align = channels as u32 * (bits_per_sample as u32 / 8);
    let byte_rate = sample_rate * block_align;
    let data_size = sample_count * block_align as u64;
    let riff_size = 36 + data_size;

    if sample_count == 0 {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .context("open wav for header init")?;
        let mut file = std::io::BufWriter::new(file);
        file.write_all(b"RIFF")?;
        file.write_all(&(36u32).to_le_bytes())?;
        file.write_all(b"WAVEfmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?; // PCM
        file.write_all(&channels.to_le_bytes())?;
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&(block_align as u16).to_le_bytes())?;
        file.write_all(&bits_per_sample.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&0u32.to_le_bytes())?;
        file.flush()?;
        file.into_inner()?.sync_all()?;
    } else {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .context("open wav for header patch")?;
        file.seek(SeekFrom::Start(4))?;
        file.write_all(&(riff_size as u32).to_le_bytes())?;
        file.seek(SeekFrom::Start(40))?;
        file.write_all(&(data_size as u32).to_le_bytes())?;
        file.sync_all()?;
    }
    Ok(())
}

/// Infer sample count from on-disk byte length and rewrite the WAV header.
pub fn repair_wav_header_from_file_size(path: &Path) -> Result<u64> {
    let len = std::fs::metadata(path).context("stat wav")?.len();
    if len <= 44 {
        return Err(anyhow!("wav file too small to repair"));
    }
    let sample_count = (len - 44) / 2;
    sync_wav_header(path, crate::services::audio::WHISPER_SAMPLE_RATE, 1, 16, sample_count)?;
    Ok(sample_count)
}

// ── Text cleanup ──────────────────────────────────────────────────────────────

/// Strip Whisper artifact annotations and normalize whitespace from a single segment.
/// Always-on — these are never valid speech output.
fn cleanup_text(text: &str) -> String {
    // Strip uppercase-first Whisper bracket annotations: [BLANK_AUDIO], [Music], [Applause], etc.
    // Requires first char uppercase so user annotations like [note] are preserved.
    let caps_re = Regex::new(r"\[[A-Z][A-Za-z_ ]*\]").expect("static regex");
    let cleaned = caps_re.replace_all(text, "");
    // Also strip known lowercase Whisper noise tokens emitted by the VAD path.
    // Named list is explicit: [note] and other user annotations are not affected.
    let noise_re = Regex::new(
        r"(?i)\[(silence|blank_audio|no_speech|music|applause|laughter|noise|inaudible)\]",
    )
    .expect("static regex");
    let cleaned = noise_re.replace_all(&cleaned, "");
    // Whisper sometimes fuses its native "#word" output with a following command word
    // (e.g. "hashtag cake new line" → "#cakenewline"). Split so replacement rules fire.
    let fusion_re = Regex::new(r"(?i)(#\w+?)(newline)").expect("static regex");
    let cleaned = fusion_re.replace_all(&cleaned, "$1 $2");
    // Normalize internal whitespace
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    words.join(" ")
}

/// Remove consecutively repeated phrases of 1–5 words (case-insensitive).
/// Handles Whisper repetition artifacts at segment boundaries:
///   "hello world. world. Next"  → "hello world. Next"
///   "eat some food eat some food" → "eat some food"
fn dedup_consecutive_phrases(text: &str) -> String {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < 2 {
        return text.to_string();
    }
    let mut result: Vec<&str> = Vec::with_capacity(tokens.len());
    result.push(tokens[0]);
    let mut i = 1;
    while i < tokens.len() {
        let mut skipped = false;
        // Longest match first so "eat some food" beats "eat" when both repeat
        for phrase_len in (1..=5).rev() {
            if i + phrase_len > tokens.len() || result.len() < phrase_len {
                continue;
            }
            let prev = &result[result.len() - phrase_len..];
            let curr = &tokens[i..i + phrase_len];
            let matches = prev.iter().zip(curr.iter()).all(|(p, c)| {
                let p = p.trim_end_matches(|ch: char| !ch.is_alphanumeric());
                let c = c.trim_end_matches(|ch: char| !ch.is_alphanumeric());
                p.eq_ignore_ascii_case(c)
            });
            if matches {
                i += phrase_len;
                skipped = true;
                break;
            }
        }
        if !skipped {
            result.push(tokens[i]);
            i += 1;
        }
    }
    result.join(" ")
}

/// When Whisper (or a double-paste bug) yields the same paragraph twice back-to-back, keep one copy.
fn dedup_exact_halves(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() < 40 {
        return trimmed.to_string();
    }
    let mid = trimmed.len() / 2;
    let (first, second) = trimmed.split_at(mid);
    let second = second.trim_start();
    if first == second {
        return first.trim().to_string();
    }
    // Allow a single missing/extra space at the join (common when segments abut).
    let a = first.trim();
    let b = second.trim();
    if a.len() >= 20 && b.starts_with(a) && b.len() <= a.len() + 2 {
        return a.to_string();
    }
    trimmed.to_string()
}

/// If the transcript appears twice (Whisper hallucination on long audio), keep the first copy.
/// Uses the opening fingerprint (~20% of text, ≤100 chars) to detect the repeat start.
fn dedup_repeated_block(text: &str) -> String {
    let min_len = 60;
    if text.len() < min_len * 2 {
        return text.to_string();
    }
    let fp_len = (text.len() / 5).clamp(20, 100);
    let fingerprint = text[..fp_len].to_lowercase();
    // Search for the repeat only in the second half
    let search_from = text.len() / 2;
    if let Some(pos) = text[search_from..].to_lowercase().find(&fingerprint) {
        return text[..search_from + pos].trim_end().to_string();
    }
    text.to_string()
}

// ── Replacement engine ────────────────────────────────────────────────────────

/// Apply user-defined replacement rules to text. Rules are applied in order.
/// Only rules whose scope is Both or matches the given scope are applied.
/// Returns the effective trigger string, prepending `prefix` when the trigger doesn't
/// already include it. This allows old-format rules ("float dash") and new-format rules
/// ("dash" with prefix="float") to coexist and produce identical behaviour.
fn effective_trigger(trigger: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return trigger.to_string();
    }
    let prefix_space = format!("{} ", prefix);
    if trigger.starts_with(&prefix_space) {
        trigger.to_string()
    } else {
        format!("{} {}", prefix, trigger)
    }
}

fn apply_replacements(
    text: &str,
    rules: &[ReplacementRule],
    scope: &ReplacementScope,
    prefix: &str,
) -> String {
    let mut result = text.to_string();
    for rule in rules {
        if rule.trigger.trim().is_empty() {
            continue;
        }
        let rule_scope = &rule.scope;
        if rule_scope != &ReplacementScope::Both && rule_scope != scope {
            continue;
        }
        let triggers: Vec<String> = std::iter::once(rule.trigger.as_str())
            .chain(rule.aliases.iter().map(String::as_str))
            .filter(|t| !t.trim().is_empty())
            .map(|t| effective_trigger(t, prefix))
            .collect();
        for trigger in &triggers {
            result = match rule.rule_type {
                ReplacementRuleType::Simple => {
                    let replacement = rule.output.as_str();
                    if trigger.contains(' ') {
                        replace_phrase(&result, trigger, replacement)
                    } else {
                        replace_whole_word(&result, trigger, replacement)
                    }
                }
                ReplacementRuleType::Newline => replace_newline(&result, trigger),
                ReplacementRuleType::Wrap => {
                    wrap_next_word(&result, trigger, &rule.prefix, &rule.suffix, &rule.transform)
                }
            };
        }
    }
    result
}

fn replace_whole_word(text: &str, trigger: &str, replacement: &str) -> String {
    let pattern = format!(r"(?i)\b{}\b", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, replacement).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn replace_phrase(text: &str, trigger: &str, replacement: &str) -> String {
    let pattern = format!(r"(?i){}", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, replacement).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn replace_newline(text: &str, trigger: &str) -> String {
    let escaped = regex::escape(trigger);
    // Optional leading space absorbs the word-gap so no trailing whitespace is left
    // on the preceding line. Optional trailing punctuation is moved before the newline
    // so Whisper-added sentence endings (e.g. "new line?") don't land on the new line.
    let pattern = if trigger.contains(' ') {
        format!(r"(?i)[ ]?{}([.,!?;:]+)?[ ]*", escaped)
    } else {
        format!(r"(?i)[ ]?\b{}\b([.,!?;:]+)?[ ]*", escaped)
    };
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, |caps: &regex::Captures| {
            let punct = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            format!("{}\n", punct)
        }).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn wrap_next_word(text: &str, trigger: &str, prefix: &str, suffix: &str, transform: &WordTransform) -> String {
    // Match: whole-word trigger + whitespace + next non-whitespace word
    let pattern = format!(r"(?i)\b{}\s+(\S+)", regex::escape(trigger));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, |caps: &regex::Captures| {
            // Strip leading non-alphanumeric chars Whisper may have already inserted
            // (e.g. "hashtag #word" → Whisper pre-pended "#", avoid "##word")
            let raw = caps[1].trim_start_matches(|c: char| !c.is_alphanumeric());
            let word = apply_word_transform(if raw.is_empty() { &caps[1] } else { raw }, transform);
            format!("{}{}{}", prefix, word, suffix)
        }).into_owned(),
        Err(_) => text.to_string(),
    }
}

fn apply_word_transform(word: &str, transform: &WordTransform) -> String {
    match transform {
        WordTransform::Lower => word.to_lowercase(),
        WordTransform::Upper => word.to_uppercase(),
        WordTransform::Sentence => {
            let lower = word.to_lowercase();
            let mut chars = lower.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
        WordTransform::None => word.to_string(),
    }
}

/// Returns "in", "out", or "" depending on the speaker label at the start of a segment.
/// Used by write_transcript to prevent merging across speaker sources.
fn speaker_source_prefix(text: &str) -> &'static str {
    if text.starts_with("in: ") { "in" }
    else if text.starts_with("out: ") { "out" }
    else { "" }
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
    use crate::types::{Note, ReplacementRule, ReplacementRuleType, ReplacementScope, WordTransform};

    fn simple_rule(trigger: &str, output: &str, scope: ReplacementScope) -> ReplacementRule {
        ReplacementRule {
            trigger: trigger.to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Simple,
            output: output.to_string(),
            scope,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        }
    }

    fn wrap_rule(trigger: &str, prefix: &str, suffix: &str, transform: WordTransform) -> ReplacementRule {
        ReplacementRule {
            trigger: trigger.to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Wrap,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
            transform,
        }
    }

    fn newline_rule(trigger: &str) -> ReplacementRule {
        ReplacementRule {
            trigger: trigger.to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Newline,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        }
    }

    // ── cleanup_text ────────────────────────────────────────────────────────

    #[test]
    fn cleanup_splits_hashtag_newline_fusion() {
        // Whisper fuses "#cake" + "newline" into one token — must be split so rules fire
        assert_eq!(cleanup_text("#cakenewline"), "#cake newline");
    }


    #[test]
    fn cleanup_strips_silence_annotation() {
        assert_eq!(cleanup_text("[SILENCE] hello"), "hello");
    }

    #[test]
    fn cleanup_strips_blank_audio_annotation() {
        assert_eq!(cleanup_text("[BLANK_AUDIO]"), "");
    }

    #[test]
    fn cleanup_strips_general_bracket_annotation() {
        assert_eq!(cleanup_text("[MUSIC] welcome back [APPLAUSE]"), "welcome back");
    }

    #[test]
    fn cleanup_preserves_lowercase_brackets() {
        // User-facing [note] or [1] should not be stripped — only ALL-CAPS Whisper annotations
        assert_eq!(cleanup_text("see [note] below"), "see [note] below");
    }

    #[test]
    fn cleanup_strips_lowercase_whisper_noise_tokens() {
        // Whisper VAD path emits [silence] in lowercase; some models emit [music], [blank_audio], etc.
        assert_eq!(cleanup_text("[silence] hello"), "hello");
        assert_eq!(cleanup_text("[blank_audio]"), "");
        assert_eq!(cleanup_text("[music] intro [applause]"), "intro");
        assert_eq!(cleanup_text("[inaudible] world"), "world");
    }

    #[test]
    fn cleanup_normalizes_whitespace() {
        assert_eq!(cleanup_text("  hello   world  "), "hello world");
    }

    // ── dedup_consecutive_phrases ─────────────────────────────────────────────

    #[test]
    fn dedup_exact_halves_removes_verbatim_repeat() {
        let once = "No, I'm using dictate right now to test.";
        let twice = format!("{once}{once}");
        assert_eq!(dedup_exact_halves(&twice), once);
    }

    #[test]
    fn dedup_removes_consecutive_duplicate_words() {
        assert_eq!(dedup_consecutive_phrases("hello world world next"), "hello world next");
    }

    #[test]
    fn dedup_case_insensitive() {
        assert_eq!(dedup_consecutive_phrases("Hello hello world"), "Hello world");
    }

    #[test]
    fn dedup_does_not_remove_non_consecutive_duplicates() {
        assert_eq!(dedup_consecutive_phrases("hello world hello"), "hello world hello");
    }

    #[test]
    fn dedup_handles_punctuation_at_word_boundary() {
        // "world." and "world" are considered duplicates (strip trailing punct for compare)
        assert_eq!(dedup_consecutive_phrases("hello world. world next"), "hello world. next");
    }

    // ── apply_replacements ──────────────────────────────────────────────────

    #[test]
    fn replacements_simple_whole_word() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Both, ""), "11 - may");
    }

    #[test]
    fn replacements_case_insensitive() {
        let rules = vec![simple_rule("hashtag", "#", ReplacementScope::Both)];
        assert_eq!(apply_replacements("HASHTAG project", &rules, &ReplacementScope::Both, ""), "# project");
    }

    #[test]
    fn replacements_whole_word_not_substring() {
        let rules = vec![simple_rule("hash", "#", ReplacementScope::Both)];
        assert_eq!(apply_replacements("hashtag project", &rules, &ReplacementScope::Both, ""), "hashtag project");
    }

    #[test]
    fn replacements_phrase_trigger() {
        let rules = vec![simple_rule("to do", "[ ]", ReplacementScope::Both)];
        assert_eq!(apply_replacements("add to do item", &rules, &ReplacementScope::Both, ""), "add [ ] item");
    }

    #[test]
    fn replacements_newline_type() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(apply_replacements("hello new line world", &rules, &ReplacementScope::Both, ""), "hello\nworld");
    }

    #[test]
    fn replace_newline_moves_trailing_question_mark() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(
            apply_replacements("looking for new line?", &rules, &ReplacementScope::Both, ""),
            "looking for?\n"
        );
    }

    #[test]
    fn replace_newline_alias_single_word() {
        let rules = vec![ReplacementRule {
            trigger: "new line".to_string(),
            aliases: vec!["newline".to_string()],
            rule_type: ReplacementRuleType::Newline,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        }];
        assert_eq!(
            apply_replacements("go to bed newline", &rules, &ReplacementScope::Both, ""),
            "go to bed\n"
        );
    }

    #[test]
    fn replacements_wrap_with_lower_transform() {
        let rules = vec![wrap_rule("hashtag", "#", "", WordTransform::Lower)];
        assert_eq!(apply_replacements("hashtag Monday", &rules, &ReplacementScope::Both, ""), "#monday");
    }

    #[test]
    fn replacements_wrap_leaves_rest_unchanged() {
        let rules = vec![wrap_rule("bold", "**", "**", WordTransform::None)];
        assert_eq!(apply_replacements("bold hello world", &rules, &ReplacementScope::Both, ""), "**hello** world");
    }

    #[test]
    fn replacements_scope_transcripts_skips_dictate_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Dictate)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Transcripts, ""), "11 dash may");
    }

    #[test]
    fn replacements_scope_dictate_skips_transcripts_rule() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Transcripts)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Dictate, ""), "11 dash may");
    }

    #[test]
    fn replacements_both_scope_applies_to_transcripts() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("a dash b", &rules, &ReplacementScope::Transcripts, ""), "a - b");
    }

    #[test]
    fn replacements_multiple_rules_in_order() {
        let rules = vec![
            simple_rule("hashtag", "#", ReplacementScope::Both),
            simple_rule("todo", "[ ]", ReplacementScope::Both),
        ];
        assert_eq!(
            apply_replacements("hashtag project todo item", &rules, &ReplacementScope::Both, ""),
            "# project [ ] item"
        );
    }

    #[test]
    fn replacements_empty_trigger_skipped() {
        let rules = vec![simple_rule("", "oops", ReplacementScope::Both)];
        assert_eq!(apply_replacements("hello", &rules, &ReplacementScope::Both, ""), "hello");
    }

    // Old-format rules (prefix embedded in trigger string) still work with empty prefix.
    #[test]
    fn old_format_rule_with_empty_prefix_matches() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("11 float dash may", &rules, &ReplacementScope::Both, ""), "11 - may");
    }

    #[test]
    fn old_format_bare_word_does_not_match() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("11 dash may", &rules, &ReplacementScope::Both, ""), "11 dash may");
    }

    #[test]
    fn old_format_newline_rule_with_empty_prefix_matches() {
        let rules = vec![newline_rule("float new line")];
        assert_eq!(apply_replacements("hello float new line world", &rules, &ReplacementScope::Both, ""), "hello\nworld");
    }

    #[test]
    fn old_format_bare_new_line_does_not_match() {
        let rules = vec![newline_rule("float new line")];
        assert_eq!(apply_replacements("hello new line world", &rules, &ReplacementScope::Both, ""), "hello new line world");
    }

    // ── Global prefix feature ──────────────────────────────────────────────────

    #[test]
    fn prefix_prepended_to_base_trigger_fires() {
        // New-format rule: base trigger "dash", prefix "float" → effective "float dash"
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("eleven float dash may", &rules, &ReplacementScope::Both, "float"), "eleven - may");
    }

    #[test]
    fn prefix_base_trigger_alone_does_not_fire() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("eleven dash may", &rules, &ReplacementScope::Both, "float"), "eleven dash may");
    }

    #[test]
    fn prefix_not_double_applied_to_old_format_rule() {
        // Old-format rule already has "float " in trigger; prefix "float" must not produce "float float dash"
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("eleven float dash may", &rules, &ReplacementScope::Both, "float"), "eleven - may");
    }

    #[test]
    fn prefix_double_prefixed_text_does_not_match_old_format() {
        let rules = vec![simple_rule("float dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("eleven float float dash may", &rules, &ReplacementScope::Both, "float"), "eleven float float dash may");
    }

    #[test]
    fn empty_prefix_fires_base_trigger_directly() {
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        assert_eq!(apply_replacements("eleven dash may", &rules, &ReplacementScope::Both, ""), "eleven - may");
    }

    #[test]
    fn prefix_newline_rule_base_trigger() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(apply_replacements("hello float new line world", &rules, &ReplacementScope::Both, "float"), "hello\nworld");
    }

    #[test]
    fn prefix_newline_bare_trigger_does_not_fire() {
        let rules = vec![newline_rule("new line")];
        assert_eq!(apply_replacements("hello new line world", &rules, &ReplacementScope::Both, "float"), "hello new line world");
    }


    fn temp_file(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liscribe-output-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join(name)
    }

    fn temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liscribe-output-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn list_transcripts_empty_folder() {
        let dir = temp_dir();
        let svc = OutputService;
        let result = svc.list_transcripts(dir.to_str().unwrap()).expect("list transcripts");
        assert!(result.is_empty());
    }

    #[test]
    fn list_transcript_metadata_reads_title_from_prefix_only() {
        let dir = temp_dir();
        let mut content = String::from("---\ntitle: 'Huge Doc'\nmodel: small\n---\n\n## Transcript\n\n");
        content.push_str(&"x".repeat(50_000));
        std::fs::write(dir.join("huge.md"), content).unwrap();
        let entries =
            OutputService::list_transcript_metadata(dir.to_str().unwrap()).expect("list metadata");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Huge Doc");
        assert_eq!(entries[0].model, "small");
    }

    #[test]
    fn list_transcripts_reads_title_and_model_from_front_matter() {
        let dir = temp_dir();
        let content = "---\ntitle: 'My Meeting'\nduration_seconds: 30.0\nword_count: 50\ntoken_estimate: 65\nmodel: tiny\n---\n\n## Transcript\n\nHello world.\n";
        std::fs::write(dir.join("my_meeting_tiny.md"), content).unwrap();
        let svc = OutputService;
        let entries = svc.list_transcripts(dir.to_str().unwrap()).expect("list transcripts");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "My Meeting");
        assert_eq!(entries[0].model, "tiny");
    }

    #[test]
    fn list_transcripts_returns_all_md_files_sorted_desc() {
        let dir = temp_dir();
        let content = |title: &str| format!("---\ntitle: '{title}'\nmodel: tiny\n---\n\n## Transcript\n\nText.\n");
        std::fs::write(dir.join("a.md"), content("Alpha")).unwrap();
        std::fs::write(dir.join("b.md"), content("Beta")).unwrap();
        let svc = OutputService;
        let entries = svc.list_transcripts(dir.to_str().unwrap()).expect("list transcripts");
        assert_eq!(entries.len(), 2);
        // entries are sorted descending; verify the invariant holds
        assert!(entries[0].modified_at >= entries[1].modified_at);
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

        svc.write_transcript(&segments, &[], "Test", "tiny", true, &[], "", &file)
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

        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write transcript");

        let content = std::fs::read_to_string(&file).expect("read transcript");
        assert!(content.contains("hello world"));
        assert!(!content.contains("[00:00:12]"));
    }

    #[test]
    fn dual_source_segments_are_never_merged_across_speaker_boundary() {
        // "in: yeah" ends at 1000 ms; "out: Hello." starts at 1200 ms — gap is only 200 ms,
        // but they must NOT merge because the source labels differ.
        let svc = OutputService;
        let file = temp_file("dual-source-newlines.md");
        let segments = vec![
            Segment { start_ms: 0, end_ms: 1_000, text: "in: yeah".to_string() },
            Segment { start_ms: 1_200, end_ms: 3_000, text: "out: Hello there.".to_string() },
            Segment { start_ms: 3_100, end_ms: 4_000, text: "out: How are you?".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        // Speaker change uses a blank line
        assert!(
            content.contains("in: yeah\n\nout: Hello there."),
            "in: and out: should be separated by a blank line, got:\n{content}"
        );
        // Two consecutive "out:" segments within gap should merge without repeating the label
        assert!(
            content.contains("out: Hello there. How are you?"),
            "consecutive out: segments within gap should merge without repeating label, got:\n{content}"
        );
    }

    #[test]
    fn dual_source_speaker_change_uses_blank_line() {
        let svc = OutputService;
        let file = temp_file("dual-source-compact.md");
        let segments = vec![
            Segment { start_ms: 0, end_ms: 1_000, text: "in: yeah".to_string() },
            Segment { start_ms: 2_000, end_ms: 4_000, text: "out: Thanks for sharing.".to_string() },
            Segment { start_ms: 5_000, end_ms: 6_000, text: "in: Absolutely.".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        // Each speaker change: blank line \n\n
        assert!(content.contains("in: yeah\n\nout:"), "in→out should be \\n\\n, got:\n{content}");
        assert!(content.contains("out: Thanks for sharing.\n\nin:"), "out→in should be \\n\\n, got:\n{content}");
    }

    #[test]
    fn single_source_always_uses_double_newline() {
        let svc = OutputService;
        let file = temp_file("single-source-separator.md");
        // Two segments with a gap > 8 s so they stay separate paragraphs
        let segments = vec![
            Segment { start_ms: 0, end_ms: 2_000, text: "First thought.".to_string() },
            Segment { start_ms: 12_000, end_ms: 14_000, text: "Second thought.".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        assert!(
            content.contains("First thought.\n\nSecond thought."),
            "single-source paragraphs should use \\n\\n, got:\n{content}"
        );
    }

    #[test]
    fn single_source_segments_still_merge_within_gap() {
        let svc = OutputService;
        let file = temp_file("single-source-merge.md");
        let segments = vec![
            Segment { start_ms: 0, end_ms: 500, text: "Hello".to_string() },
            Segment { start_ms: 700, end_ms: 1_200, text: "world.".to_string() },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        assert!(
            content.contains("Hello world."),
            "same-source segments within gap should merge, got:\n{content}"
        );
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

    // ── renderer extraction / parity ─────────────────────────────────────────

    #[test]
    fn render_transcript_markdown_golden_dual_source_notes_rules_timestamps() {
        // Locks the exact byte output of the extracted renderer: dual-source grouping,
        // a replacement rule, timestamps on, and a notes section.
        let segments = vec![
            Segment { start_ms: 0, end_ms: 1_000, text: "in: hello dash world".to_string() },
            Segment { start_ms: 1_200, end_ms: 3_000, text: "out: How are you?".to_string() },
            Segment { start_ms: 3_100, end_ms: 4_000, text: "out: I am well.".to_string() },
        ];
        let notes = vec![Note {
            id: "n1".to_string(),
            text: "follow up".to_string(),
            recorded_at_ms: 2_000,
        }];
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        let md = render_transcript_markdown(&segments, &notes, "My Title", "tiny", true, &rules, "");
        // word_count counts every whitespace token of the timestamp-free body, including the
        // `in:`/`out:` speaker labels and the substituted `-` (11 tokens). A notes section ends
        // with a blank line (the loop's trailing `\n` plus the document's final `\n`).
        let expected = "---\n\
title: 'My Title'\n\
duration_seconds: 4.0\n\
word_count: 11\n\
token_estimate: 14\n\
model: tiny\n\
---\n\n\
## Transcript\n\n\
[00:00:00] in: hello - world\n\n\
[00:00:01] out: How are you? I am well.\n\n\
## Notes\n\
[1] (00:00:02) follow up\n\n";
        assert_eq!(md, expected);
    }

    #[test]
    fn write_transcript_matches_pure_renderer() {
        let svc = OutputService;
        let file = temp_file("parity.md");
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 2_000,
            text: "hello world".to_string(),
        }];
        svc.write_transcript(&segments, &[], "T", "tiny", false, &[], "", &file)
            .expect("write");
        let on_disk = std::fs::read_to_string(&file).expect("read");
        let pure = render_transcript_markdown(&segments, &[], "T", "tiny", false, &[], "");
        assert_eq!(on_disk, pure);
    }

    #[test]
    fn count_words_excludes_timestamp_labels() {
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
        }];
        // Two real words regardless of timestamp rendering.
        assert_eq!(count_words(&segments, &[], ""), 2);
    }

    // ── format_ms ────────────────────────────────────────────────────────────

    #[test]
    fn format_ms_zero() {
        assert_eq!(format_ms(0), "00:00:00");
    }

    #[test]
    fn format_ms_seconds_only() {
        assert_eq!(format_ms(12_000), "00:00:12");
    }

    #[test]
    fn format_ms_minutes_and_seconds() {
        assert_eq!(format_ms(90_000), "00:01:30");
    }

    #[test]
    fn format_ms_hours_minutes_seconds() {
        assert_eq!(format_ms(3_661_000), "01:01:01");
    }

    #[test]
    fn format_ms_pads_single_digits() {
        assert_eq!(format_ms(3_600_000 + 60_000 + 5_000), "01:01:05");
    }

    // ── transcript_path slug ─────────────────────────────────────────────────

    #[test]
    fn transcript_path_replaces_spaces_with_underscores() {
        let svc = OutputService;
        let dir = std::env::temp_dir();
        let model = std::path::Path::new("/models/ggml-tiny.bin");
        let path = svc.transcript_path(&dir, model, "my title");
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("my_title_"));
    }

    #[test]
    fn transcript_path_replaces_forbidden_chars_with_dashes() {
        let svc = OutputService;
        let dir = std::env::temp_dir();
        let model = std::path::Path::new("/models/ggml-tiny.bin");
        let path = svc.transcript_path(&dir, model, "foo/bar:baz");
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("foo-bar-baz_"));
    }

    #[test]
    fn transcript_path_uses_save_folder_root_without_suffix_when_free() {
        let svc = OutputService;
        let dir = temp_save_folder();
        let model = std::path::Path::new("/models/ggml-small.en-q5_1.bin");
        let path = svc.transcript_path(Path::new(&dir), model, "Standup");
        assert_eq!(path.parent().unwrap(), Path::new(&dir));
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "Standup_ggml-small.en-q5_1.md");
    }

    #[test]
    fn transcript_path_appends_numeric_suffix_on_collision() {
        let svc = OutputService;
        let dir = temp_save_folder();
        let folder = Path::new(&dir);
        let model = std::path::Path::new("/models/ggml-small.en-q5_1.bin");
        let first = svc.transcript_path(folder, model, "Standup");
        std::fs::write(&first, "# first").expect("seed first transcript");

        let second = svc.transcript_path(folder, model, "Standup");
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            "Standup_ggml-small.en-q5_1_1.md"
        );

        std::fs::write(&second, "# second").expect("seed second transcript");
        let third = svc.transcript_path(folder, model, "Standup");
        assert_eq!(
            third.file_name().unwrap().to_string_lossy(),
            "Standup_ggml-small.en-q5_1_2.md"
        );
    }

    // ── write_dictate_history_entry / read_dictate_history ───────────────────

    fn temp_save_folder() -> String {
        let dir = std::env::temp_dir()
            .join(format!("liscribe-dictate-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn read_dictate_history_returns_empty_when_file_missing() {
        let svc = OutputService;
        let folder = temp_save_folder();
        let entries = svc.read_dictate_history(&folder).expect("read");
        assert!(entries.is_empty());
    }

    #[test]
    fn read_dictate_history_parses_legacy_file() {
        // Legacy dictate_history.json is read-only now; verify the parser still handles it.
        let svc = OutputService;
        let folder = temp_save_folder();
        let json = r#"[{"id":"a1","timestamp":"2026-01-01T00:00:00Z","text":"second"},
                       {"id":"a0","timestamp":"2025-12-31T00:00:00Z","text":"first"}]"#;
        std::fs::write(PathBuf::from(&folder).join("dictate_history.json"), json).unwrap();

        let entries = svc.read_dictate_history(&folder).expect("read");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second");
        assert_eq!(entries[1].id, "a0");
    }

    // ── sync_wav_header / session manifest / salvage ─────────────────────────

    #[test]
    fn streaming_wav_placeholder_is_valid_wav() {
        let dir = std::env::temp_dir().join(format!("wav-ph-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("placeholder.wav");
        write_streaming_wav_placeholder(&path, 16_000, 1, 16).unwrap();
        let reader = hound::WavReader::open(&path).expect("open placeholder");
        assert_eq!(reader.spec().bits_per_sample, 16);
    }

    #[test]
    fn sync_wav_header_makes_checkpointed_wav_readable() {
        use std::io::Write;

        let dir = std::env::temp_dir().join(format!("wav-sync-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("partial.wav");
        write_streaming_wav_placeholder(&path, 16_000, 1, 16).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&vec![0u8; 3200]).unwrap();
        sync_wav_header(&path, 16_000, 1, 16, 1600).unwrap();
        let pcm = crate::services::audio::read_wav_mono_f32(&path).expect("read checkpointed wav");
        assert_eq!(pcm.len(), 1600);
    }

    #[test]
    fn repair_wav_header_from_file_size_roundtrip() {
        use std::io::Write;

        let dir = std::env::temp_dir().join(format!("wav-repair-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("truncated.wav");
        write_streaming_wav_placeholder(&path, 16_000, 1, 16).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&vec![0u8; 6400]).unwrap();
        repair_wav_header_from_file_size(&path).expect("repair");
        let pcm = crate::services::audio::read_wav_mono_f32(&path).expect("read repaired wav");
        assert_eq!(pcm.len(), 3200);
    }

    #[test]
    fn write_session_manifest_roundtrip() {
        use crate::types::{SessionManifest, SessionManifestState};

        let svc = OutputService;
        let dir = std::env::temp_dir().join(format!("session-manifest-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = SessionManifest {
            format_version: 1,
            state: SessionManifestState::Recording,
            started_at: "2026-05-28T12:00:00Z".to_string(),
            mic_wav: "mic.wav".to_string(),
            speaker_wavs: vec!["speaker_seg_0.wav".to_string()],
            transcript_path: None,
            title: None,
        };
        svc.write_session_manifest(&dir, &manifest).expect("write");
        let raw = std::fs::read_to_string(dir.join("session.json")).expect("read");
        let parsed: SessionManifest = serde_json::from_str(&raw).expect("parse");
        assert_eq!(parsed.state, SessionManifestState::Recording);
        assert_eq!(parsed.speaker_wavs, vec!["speaker_seg_0.wav".to_string()]);
    }

    #[test]
    fn salvage_dictate_wav_moves_to_failures_dir() {
        let svc = OutputService;
        let save_folder = temp_save_folder();
        let temp_wav = PathBuf::from(&save_folder).join("temp_capture.wav");
        svc.write_wav(&[0.0; 1600], 16_000, &temp_wav).expect("write temp wav");
        assert!(temp_wav.is_file());

        let dest = svc
            .salvage_dictate_wav(&save_folder, &temp_wav)
            .expect("salvage");
        assert!(dest.is_file());
        assert!(dest.to_string_lossy().contains("dictate_failures"));
        assert!(!temp_wav.exists());
    }

    #[test]
    fn scan_incomplete_scribe_sessions_finds_non_complete_manifest() {
        use crate::types::{SessionManifest, SessionManifestState};

        let svc = OutputService;
        let save_folder = temp_save_folder();
        let session_dir = PathBuf::from(&save_folder).join("2026-05-28_12-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        svc.write_wav(&[0.0; 800], 16_000, &session_dir.join("mic.wav"))
            .expect("write mic");
        svc.write_session_manifest(
            &session_dir,
            &SessionManifest {
                format_version: 1,
                state: SessionManifestState::Recording,
                started_at: "2026-05-28T12:00:00Z".to_string(),
                mic_wav: "mic.wav".to_string(),
                speaker_wavs: vec![],
                transcript_path: None,
                title: None,
            },
        )
        .expect("write manifest");

        let found = svc
            .scan_incomplete_scribe_sessions(&save_folder)
            .expect("scan");
        assert_eq!(found.len(), 1);
        assert!(found[0].session_dir.ends_with("2026-05-28_12-00-00"));
    }

    #[test]
    fn remove_session_dir_deletes_all_staging_files() {
        let svc = OutputService;
        let dir = std::env::temp_dir().join(format!("remove-session-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("session.json"), b"{}").unwrap();
        svc.write_wav(&[0.0; 400], 16_000, &dir.join("mic.wav")).expect("write mic");

        svc.remove_session_dir(&dir);
        assert!(!dir.exists());
    }

    #[test]
    fn finalize_scribe_session_removes_dir_when_keep_wav_off() {
        let svc = OutputService;
        let save_folder = temp_save_folder();
        let session_dir = PathBuf::from(&save_folder).join("2026-05-28_13-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("session.json"), b"{}").unwrap();
        svc.write_wav(&[0.0; 400], 16_000, &session_dir.join("mic.wav"))
            .expect("write mic");
        let transcript = PathBuf::from(&save_folder).join("note_tiny.md");
        std::fs::write(&transcript, b"# test").unwrap();

        svc.finalize_scribe_session(&session_dir, false).expect("finalize");

        assert!(transcript.is_file());
        assert!(!session_dir.exists());
    }

    #[test]
    fn finalize_scribe_session_keeps_wavs_when_keep_wav_on() {
        let svc = OutputService;
        let save_folder = temp_save_folder();
        let session_dir = PathBuf::from(&save_folder).join("2026-05-28_14-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("session.json"), b"{}").unwrap();
        std::fs::write(session_dir.join("notes.json"), b"{}").unwrap();
        svc.write_wav(&[0.0; 400], 16_000, &session_dir.join("mic.wav"))
            .expect("write mic");

        svc.finalize_scribe_session(&session_dir, true).expect("finalize");

        assert!(session_dir.is_dir());
        assert!(session_dir.join("mic.wav").is_file());
        assert!(!session_dir.join("session.json").exists());
        assert!(!session_dir.join("notes.json").exists());
    }

    #[test]
    fn recovery_scan_skips_wav_only_dirs_without_manifest() {
        let svc = OutputService;
        let save_folder = temp_save_folder();
        let session_dir = PathBuf::from(&save_folder).join("2026-05-28_15-00-00");
        std::fs::create_dir_all(&session_dir).unwrap();
        svc.write_wav(&[0.0; 400], 16_000, &session_dir.join("mic.wav"))
            .expect("write mic");

        let found = svc
            .scan_incomplete_scribe_sessions(&save_folder)
            .expect("scan");
        assert!(found.is_empty());
    }
}
