use crate::types::{
    DictateHistoryEntry, Note, RecoverySessionInfo, ReplacementRule, ReplacementScope,
    ScribeTranscriptEntry, Segment, SessionManifest, SpeakerBlock,
};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod cleanup;
mod dedup;
pub mod hallucination;
mod legacy;
mod render;
mod replacements;
mod session;
pub mod wav;

pub use hallucination::{
    filter_hallucination_phrases, speaker_pcm_has_signal, SPEAKER_SILENCE_THRESHOLD,
};
pub use render::{count_words, render_speaker_blocks_body, render_transcript_body};
pub use wav::{sync_wav_header, write_streaming_wav_placeholder};

pub struct OutputService;

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
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
    } else {
        slug
    };
    let stem = model_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "model".to_string());
    format!("{slug}_{stem}")
}

impl OutputService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn make_session_dir(&self, save_folder: &str) -> Result<PathBuf> {
        session::make_session_dir(save_folder)
    }

    /// Build a transcript path in the save folder root: `{save_folder}/{title}_{model}.md`.
    /// When that file already exists, appends `_1`, `_2`, … before `.md`.
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
        wav::write_wav(pcm, sample_rate, dest)
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
            .map(|s| cleanup::cleanup_text(s.text.trim()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let deduped = dedup::dedup_repeated_block(&dedup::dedup_consecutive_phrases(
            &dedup::dedup_exact_halves(&joined),
        ));
        replacements::apply_replacements(&deduped, rules, &ReplacementScope::Dictate, prefix)
    }

    /// Render segments as markdown and write. Verifies file is non-empty before returning Ok.
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
        let md = render::render_transcript_markdown(
            segments,
            notes,
            title,
            model_name,
            include_timestamps,
            rules,
            prefix,
        );
        std::fs::write(dest, &md).context("failed to write transcript")?;
        if std::fs::metadata(dest)?.len() == 0 {
            return Err(anyhow!("transcript was written empty"));
        }
        Ok(dest.to_path_buf())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_speaker_blocks_transcript(
        &self,
        blocks: &[SpeakerBlock],
        title: &str,
        model_name: &str,
        rules: &[ReplacementRule],
        prefix: &str,
        input_label: &str,
        output_label: &str,
        dest: &Path,
    ) -> Result<PathBuf> {
        let body =
            render::render_speaker_blocks_body(blocks, rules, prefix, input_label, output_label);
        let word_count = body.split_whitespace().count();
        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("title: '{}'\n", title.replace('\'', "''")));
        md.push_str(&format!("word_count: {word_count}\n"));
        md.push_str(&format!("model: {model_name}\n"));
        md.push_str("---\n\n## Transcript\n\n");
        md.push_str(&body);
        md.push('\n');
        std::fs::write(dest, &md).context("failed to write speaker transcript")?;
        if std::fs::metadata(dest)?.len() == 0 {
            return Err(anyhow!("speaker transcript was written empty"));
        }
        Ok(dest.to_path_buf())
    }

    /// Delete a single file. Silent no-op if it no longer exists.
    pub fn delete_file(&self, path: &Path) -> Result<()> {
        legacy::delete_file(path)
    }

    /// Delete a WAV file. Silent no-op if it no longer exists.
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
        session::write_session_notes(session_dir, title, wav_file_name, notes)
    }

    /// Read a transcript file.
    pub fn read_transcript(&self, path: &Path) -> Result<String, String> {
        legacy::read_transcript(path)
    }

    /// Create the directory at `path` (and missing parents) and return its canonical form.
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

    /// Open a file with the OS default handler (or a named app).
    pub fn open_file_for_user(&self, path: &str, app: Option<&str>) -> Result<(), String> {
        crate::platform::open_file(path, app)
    }

    /// Read all entries from `{save_folder}/dictate_history.json` (newest-first).
    pub fn read_dictate_history(&self, save_folder: &str) -> Result<Vec<DictateHistoryEntry>> {
        legacy::read_dictate_history(save_folder)
    }

    /// Scan `save_folder` root for `*.md` files and return their metadata sorted newest-first.
    pub fn list_transcripts(&self, save_folder: &str) -> Result<Vec<ScribeTranscriptEntry>> {
        Self::list_transcript_metadata(save_folder)
    }

    /// Same as [`list_transcripts`](Self::list_transcripts) — bounded read for History list performance.
    pub fn list_transcript_metadata(save_folder: &str) -> Result<Vec<ScribeTranscriptEntry>> {
        legacy::list_transcript_metadata(save_folder)
    }

    /// Simulate Cmd/Ctrl+V into the currently focused application.
    pub fn paste_text(&self) -> Result<(), String> {
        crate::platform::paste_impl::paste_text()
    }

    /// Simulate pressing Enter in the currently focused application.
    pub fn send_enter(&self) -> Result<(), String> {
        crate::platform::paste_impl::send_enter()
    }

    /// Best-effort delete of all files in a Scribe staging dir, then the directory itself.
    pub fn remove_session_dir(&self, dir: &Path) {
        session::remove_session_dir(dir)
    }

    /// After a successful Scribe transcription: drop `session.json` (and `notes.json`).
    pub fn finalize_scribe_session(&self, session_dir: &Path, keep_wav: bool) -> Result<()> {
        session::finalize_scribe_session(session_dir, keep_wav)
    }

    /// Write or replace `{session_dir}/session.json` for Scribe lifecycle tracking.
    pub fn write_session_manifest(
        &self,
        session_dir: &Path,
        manifest: &SessionManifest,
    ) -> Result<()> {
        session::write_session_manifest(session_dir, manifest)
    }

    /// Move a failed dictate capture into `{save_folder}/dictate_failures/{timestamp}.wav`.
    pub fn salvage_dictate_wav(&self, save_folder: &str, source_wav: &Path) -> Result<PathBuf> {
        legacy::salvage_dictate_wav(save_folder, source_wav)
    }

    /// Scan `save_folder` for Scribe sessions that did not reach `complete` and repair WAV headers.
    pub fn scan_incomplete_scribe_sessions(
        &self,
        save_folder: &str,
    ) -> Result<Vec<RecoverySessionInfo>> {
        legacy::scan_incomplete_scribe_sessions(save_folder)
    }

    /// Salvage orphaned dictate temp WAVs after a crash (checkpointed headers).
    pub fn scan_and_salvage_dictate_temp_wavs(
        &self,
        temp_dir: &Path,
        save_folder: &str,
    ) -> Result<Vec<PathBuf>> {
        legacy::scan_and_salvage_dictate_temp_wavs(temp_dir, save_folder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Note, ReplacementRule, ReplacementRuleType, ReplacementScope, Segment, SegmentSource,
        SessionManifest, SessionManifestState, WordTransform,
    };

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

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("output-mod-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.join(name)
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("output-mod-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn temp_save_folder() -> String {
        let dir = std::env::temp_dir().join(format!("output-mod-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.to_string_lossy().to_string()
    }

    // ── transcript_path ──────────────────────────────────────────────────────

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
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            "Standup_ggml-small.en-q5_1.md"
        );
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

    // ── write_transcript (integration) ───────────────────────────────────────

    #[test]
    fn transcript_renders_timestamps_when_enabled() {
        let svc = OutputService;
        let file = temp_file("with-timestamps.md");
        let segments = vec![Segment {
            start_ms: 12_000,
            end_ms: 14_000,
            text: "hello world".to_string(),
            source: None,
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
            source: None,
        }];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write transcript");
        let content = std::fs::read_to_string(&file).expect("read transcript");
        assert!(content.contains("hello world"));
        assert!(!content.contains("[00:00:12]"));
    }

    #[test]
    fn dual_source_segments_are_never_merged_across_speaker_boundary() {
        let svc = OutputService;
        let file = temp_file("dual-source-newlines.md");
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "yeah".to_string(),
                source: Some(SegmentSource::Mic),
            },
            Segment {
                start_ms: 1_200,
                end_ms: 3_000,
                text: "Hello there.".to_string(),
                source: Some(SegmentSource::Speaker),
            },
            Segment {
                start_ms: 3_100,
                end_ms: 4_000,
                text: "How are you?".to_string(),
                source: Some(SegmentSource::Speaker),
            },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        assert!(
            content.contains("yeah\n\nHello there."),
            "mic and speaker paragraphs should be separated by a blank line, got:\n{content}"
        );
        assert!(
            content.contains("Hello there. How are you?"),
            "consecutive speaker segments within gap should merge, got:\n{content}"
        );
    }

    #[test]
    fn dual_source_speaker_change_uses_blank_line() {
        let svc = OutputService;
        let file = temp_file("dual-source-compact.md");
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "yeah".to_string(),
                source: Some(SegmentSource::Mic),
            },
            Segment {
                start_ms: 2_000,
                end_ms: 4_000,
                text: "Thanks for sharing.".to_string(),
                source: Some(SegmentSource::Speaker),
            },
            Segment {
                start_ms: 5_000,
                end_ms: 6_000,
                text: "Absolutely.".to_string(),
                source: Some(SegmentSource::Mic),
            },
        ];
        svc.write_transcript(&segments, &[], "Test", "tiny", false, &[], "", &file)
            .expect("write");
        let content = std::fs::read_to_string(&file).expect("read");
        assert!(
            content.contains("yeah\n\nThanks for sharing."),
            "mic→speaker should be \\n\\n, got:\n{content}"
        );
        assert!(
            content.contains("Thanks for sharing.\n\nAbsolutely."),
            "speaker→mic should be \\n\\n, got:\n{content}"
        );
    }

    #[test]
    fn single_source_always_uses_double_newline() {
        let svc = OutputService;
        let file = temp_file("single-source-separator.md");
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 2_000,
                text: "First thought.".to_string(),
                source: None,
            },
            Segment {
                start_ms: 12_000,
                end_ms: 14_000,
                text: "Second thought.".to_string(),
                source: None,
            },
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
            Segment {
                start_ms: 0,
                end_ms: 500,
                text: "Hello".to_string(),
                source: None,
            },
            Segment {
                start_ms: 700,
                end_ms: 1_200,
                text: "world.".to_string(),
                source: None,
            },
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
    fn write_transcript_matches_pure_renderer() {
        let svc = OutputService;
        let file = temp_file("parity.md");
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 2_000,
            text: "hello world".to_string(),
            source: None,
        }];
        svc.write_transcript(&segments, &[], "T", "tiny", false, &[], "", &file)
            .expect("write");
        let on_disk = std::fs::read_to_string(&file).expect("read");
        let pure = render::render_transcript_markdown(&segments, &[], "T", "tiny", false, &[], "");
        assert_eq!(on_disk, pure);
    }

    // ── list_transcripts ──────────────────────────────────────────────────────

    #[test]
    fn list_transcripts_empty_folder() {
        let svc = OutputService;
        let dir = temp_dir();
        let result = svc
            .list_transcripts(dir.to_str().unwrap())
            .expect("list transcripts");
        assert!(result.is_empty());
    }

    #[test]
    fn list_transcripts_reads_title_and_model_from_front_matter() {
        let svc = OutputService;
        let dir = temp_dir();
        let content = "---\ntitle: 'My Meeting'\nduration_seconds: 30.0\nword_count: 50\ntoken_estimate: 65\nmodel: tiny\n---\n\n## Transcript\n\nHello world.\n";
        std::fs::write(dir.join("my_meeting_tiny.md"), content).unwrap();
        let entries = svc
            .list_transcripts(dir.to_str().unwrap())
            .expect("list transcripts");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "My Meeting");
        assert_eq!(entries[0].model, "tiny");
    }

    #[test]
    fn list_transcripts_returns_all_md_files_sorted_desc() {
        let svc = OutputService;
        let dir = temp_dir();
        let content = |title: &str| {
            format!("---\ntitle: '{title}'\nmodel: tiny\n---\n\n## Transcript\n\nText.\n")
        };
        std::fs::write(dir.join("a.md"), content("Alpha")).unwrap();
        std::fs::write(dir.join("b.md"), content("Beta")).unwrap();
        let entries = svc
            .list_transcripts(dir.to_str().unwrap())
            .expect("list transcripts");
        assert_eq!(entries.len(), 2);
        assert!(entries[0].modified_at >= entries[1].modified_at);
    }

    // ── salvage / scan / session lifecycle (OutputService delegation) ─────────

    #[test]
    fn salvage_dictate_wav_moves_to_failures_dir() {
        let svc = OutputService;
        let save_folder = temp_save_folder();
        let temp_wav = PathBuf::from(&save_folder).join("temp_capture.wav");
        svc.write_wav(&[0.0; 1600], 16_000, &temp_wav)
            .expect("write temp wav");
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
                speaker_cuts: vec![],
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

    #[test]
    fn session_notes_json_includes_title_wav_and_note_text() {
        let svc = OutputService;
        let dir = temp_dir();
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

    #[test]
    fn remove_session_dir_deletes_all_staging_files() {
        let svc = OutputService;
        let dir = temp_dir();
        std::fs::write(dir.join("session.json"), b"{}").unwrap();
        svc.write_wav(&[0.0; 400], 16_000, &dir.join("mic.wav"))
            .expect("write mic");
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
        svc.finalize_scribe_session(&session_dir, false)
            .expect("finalize");
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
        svc.finalize_scribe_session(&session_dir, true)
            .expect("finalize");
        assert!(session_dir.is_dir());
        assert!(session_dir.join("mic.wav").is_file());
        assert!(!session_dir.join("session.json").exists());
        assert!(!session_dir.join("notes.json").exists());
    }

    #[test]
    fn write_session_manifest_roundtrip() {
        let svc = OutputService;
        let dir = temp_dir();
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
        svc.write_session_manifest(&dir, &manifest).expect("write");
        let raw = std::fs::read_to_string(dir.join("session.json")).expect("read");
        let parsed: SessionManifest = serde_json::from_str(&raw).expect("parse");
        assert_eq!(parsed.state, SessionManifestState::Recording);
        assert_eq!(parsed.speaker_wavs, vec!["speaker_seg_0.wav".to_string()]);
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

    #[test]
    fn list_transcript_metadata_reads_title_from_prefix_only() {
        let dir = temp_dir();
        let mut content =
            String::from("---\ntitle: 'Huge Doc'\nmodel: small\n---\n\n## Transcript\n\n");
        content.push_str(&"x".repeat(50_000));
        std::fs::write(dir.join("huge.md"), content).unwrap();
        let entries =
            OutputService::list_transcript_metadata(dir.to_str().unwrap()).expect("list metadata");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Huge Doc");
        assert_eq!(entries[0].model, "small");
    }

    // ── simple_rule is used in a test here too ────────────────────────────────

    #[test]
    fn format_dictate_text_applies_rules() {
        let svc = OutputService;
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 1_000,
            text: "hello dash world".to_string(),
            source: None,
        }];
        let rules = vec![simple_rule("dash", "-", ReplacementScope::Both)];
        let result = svc.format_dictate_text(&segments, &rules, "");
        assert_eq!(result, "hello - world");
    }
}
