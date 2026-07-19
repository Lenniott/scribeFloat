use serde::{Deserialize, Serialize};

/// Typed error returned at the Tauri IPC boundary.
/// The `code` tag lets the frontend branch on error kind without string-matching.
/// Variants not yet used in Rust are kept so the TypeScript union type stays complete.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    NotFound(String),
    InvalidInput(String),
    StateMachine(String),
    Io(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m)
            | Self::InvalidInput(m)
            | Self::StateMachine(m)
            | Self::Io(m)
            | Self::Internal(m) => f.write_str(m),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_save_folder")]
    pub save_folder: String,

    #[serde(default)]
    pub keep_wav: bool,

    #[serde(default = "default_true")]
    pub include_timestamps: bool,

    #[serde(default = "default_open_scribe_hotkey")]
    pub open_scribe_hotkey: String,

    #[serde(default = "default_dictate_hotkey")]
    pub dictate_hotkey: String,

    #[serde(default = "default_input_label")]
    pub input_label: String,

    #[serde(default = "default_output_label")]
    pub output_label: String,

    /// Whether Scribe should capture speaker/system audio alongside mic.
    #[serde(default)]
    pub scribe_capture_speaker: bool,

    /// Preferred microphone device name for Scribe.
    /// None = use system default input.
    #[serde(default)]
    pub preferred_input_device: Option<String>,

    /// Preferred speaker-capture device name for Scribe.
    /// None = use system/default platform route.
    #[serde(default)]
    pub preferred_speaker_device: Option<String>,

    /// UI theme preference. `System` follows the OS preference.
    #[serde(default)]
    pub theme_mode: ThemeMode,

    /// Application to open transcripts with. None = system default.
    /// macOS: app name (e.g. "Obsidian"). Windows: full path to exe.
    #[serde(default)]
    pub open_with_app_path: Option<String>,

    #[serde(default)]
    pub onboarding_complete: bool,

    /// Simulate Cmd/Ctrl+V into the focused input after dictation.
    /// Requires Accessibility permission on macOS.
    #[serde(default = "default_true")]
    pub dictate_auto_paste: bool,

    /// Simulate Enter after paste (useful for chat/message apps). Default off.
    #[serde(default)]
    pub dictate_auto_enter: bool,

    /// When true, Scribe and Transcribe also write a derived `.md` next to the canonical
    /// history record. Default OFF — markdown is opt-in; the JSONL store is the source of
    /// truth. Dictate never writes `.md` regardless of this flag.
    #[serde(default)]
    pub save_transcripts_as_markdown: bool,

    /// Display name for the user in speaker-labelled transcripts. Default: "You".
    #[serde(default = "default_user_display_name")]
    pub user_display_name: String,
    // Retired keys ignored on read and dropped on the next config write:
    // - voiceprint: voice_similarity_threshold, voice_learning_enabled,
    //   voice_embeddings_retention, voice_embeddings_encryption_required
    // - multi-model chooser: selected_model_id, scribe_model_path, dictate_model_id
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_folder: default_save_folder(),
            keep_wav: false,
            include_timestamps: true,
            open_scribe_hotkey: default_open_scribe_hotkey(),
            dictate_hotkey: default_dictate_hotkey(),
            input_label: default_input_label(),
            output_label: default_output_label(),
            scribe_capture_speaker: false,
            preferred_input_device: None,
            preferred_speaker_device: None,
            theme_mode: ThemeMode::System,
            open_with_app_path: None,
            onboarding_complete: false,
            dictate_auto_paste: true,
            dictate_auto_enter: false,
            save_transcripts_as_markdown: false,
            user_display_name: default_user_display_name(),
        }
    }
}

fn default_user_display_name() -> String {
    "You".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemeMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "system" => Ok(Self::System),
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            other => Err(format!("unsupported theme mode `{other}`")),
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(target_os = "windows")]
fn default_save_folder() -> String {
    std::env::var("USERPROFILE")
        .map(|h| format!(r"{h}\Documents\transcripts_scribefloat"))
        .unwrap_or_else(|_| r"C:\Users\Public\Documents\transcripts_scribefloat".to_string())
}

#[cfg(not(target_os = "windows"))]
fn default_save_folder() -> String {
    std::env::var("HOME")
        .map(|h| format!("{h}/Documents/transcripts_scribefloat"))
        .unwrap_or_else(|_| "/tmp/transcripts_scribefloat".to_string())
}

fn default_open_scribe_hotkey() -> String {
    crate::platform::default_open_scribe_hotkey().to_string()
}

fn default_dictate_hotkey() -> String {
    crate::platform::default_dictate_activation_key().to_string()
}

fn default_input_label() -> String {
    "Mic".to_string()
}

fn default_output_label() -> String {
    "Speaker".to_string()
}

/// Stored speaker-block labels for dual-source channel tier (tier 2).
pub const CHANNEL_LABEL_IN: &str = "In";
/// Stored speaker-block labels for dual-source channel tier (tier 2).
pub const CHANNEL_LABEL_OUT: &str = "Out";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SegmentSource {
    Mic,
    Speaker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SegmentSource>,
}

impl Segment {
    pub fn new(start_ms: i64, end_ms: i64, text: impl Into<String>) -> Self {
        Self {
            start_ms,
            end_ms,
            text: text.into(),
            source: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerBlock {
    pub label: String,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
}

/// Legacy chunk-tier speaker evidence, kept only so pre-diarization notes stay
/// readable (labels + correction badges). New notes never write these; the
/// biometric fields old records carried (embeddings, scores, quality metrics)
/// are intentionally absent and dropped on the next compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerChunk {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub label: String,
    /// Label correction history, oldest first. User corrections have
    /// `auto: false`; cascade relabels triggered by them have `auto: true`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corrections: Vec<LabelCorrection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LabelCorrection {
    pub from_label: String,
    pub to_label: String,
    pub corrected_at_ms: u64,
    pub auto: bool,
}

/// Legacy transcript-local speaker group, kept only so pre-diarization notes
/// stay readable. New notes never write these; old records' centroid
/// embeddings and quality metrics are dropped on read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSpeaker {
    pub session_speaker_id: String,
    pub label: String,
    #[serde(default)]
    pub start_ms: u64,
    #[serde(default)]
    pub end_ms: u64,
    #[serde(default)]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub recorded_at_ms: u64,
}

// ── Live audio analysis (pitch / loudness change cuts) ──────────────────────────

/// Windowed pitch/loudness timeline over the 16 kHz mono mic stream.
/// Frame `i` is centered at `(i * hop_samples + window_samples / 2) / sample_rate`
/// seconds. Parallel arrays keep the JSON compact (~5× smaller than per-frame objects).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub format_version: u8,
    pub sample_rate: u32,
    pub window_samples: u32,
    pub hop_samples: u32,
    /// `None` = unvoiced frame (no pitch in the 65–400 Hz voice band).
    pub f0_hz: Vec<Option<f32>>,
    pub rms: Vec<f32>,
}

/// Why a change cut fired. Ordering matters: `BTreeSet<CutReason>` keeps
/// serialized reason lists deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CutReason {
    Pitch,
    Loudness,
    Silence,
}

/// A detected voice-change boundary. Says "the voice changed here" — spans between
/// cuts are NOT speaker identities (anonymous slots come from Sortformer diarization).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerChangeCut {
    pub time_s: f32,
    /// Last merged candidate's time; equals `time_s` for an unmerged cut.
    pub end_s: f32,
    /// Observed jump / threshold, so >= 1.0 by construction; max over merged candidates.
    pub score: f32,
    pub reasons: std::collections::BTreeSet<CutReason>,
}

/// Anonymous "who spoke when" span from Sortformer diarization, in ms since
/// capture start. Carries no identity — `speaker_id` is a per-recording slot
/// (0..=3; the model separates at most 4 voices).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiarizationRange {
    pub speaker_id: u8,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScribeState {
    Idle,
    Recording,
    Transcribing,
    Done,
    NoModel,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeStateEvent {
    pub state: ScribeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_stage: Option<ProcessingStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wav_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessingStage {
    /// Audio finalize/merge (Record, Dictate) or per-item decode (Upload) —
    /// the pre-model wait the UI must not blame on model loading.
    PreparingAudio,
    LoadingModel,
    TranscribingAudio,
    WritingTranscript,
    CleaningUpAudio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TranscribeState {
    Idle,
    Transcribing,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TranscribeItemStatus {
    Queued,
    Processing,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscribeSourceType {
    SingleAudio,
    DualSourceSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeQueueItem {
    pub id: String,
    pub source_path: String,
    pub display_name: String,
    pub source_type: TranscribeSourceType,
    pub duration_ms: u64,
    pub status: TranscribeItemStatus,
    pub progress: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeStateEvent {
    pub state: TranscribeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_stage: Option<ProcessingStage>,
    pub total_items: usize,
    pub completed_items: usize,
    pub failed_items: usize,
    pub items: Vec<TranscribeQueueItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TranscribeStateEvent {
    pub fn new(state: TranscribeState, items: Vec<TranscribeQueueItem>) -> Self {
        Self {
            state,
            progress: None,
            processing_stage: None,
            total_items: items.len(),
            completed_items: items
                .iter()
                .filter(|item| item.status == TranscribeItemStatus::Done)
                .count(),
            failed_items: items
                .iter()
                .filter(|item| item.status == TranscribeItemStatus::Error)
                .count(),
            items,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub kind: String,
    pub granted: bool,
    pub can_request: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DictateState {
    Idle,
    Recording,
    Transcribing,
    Pasting,
    /// Transcription complete, text pasted. Window stays visible briefly.
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionManifestState {
    Recording,
    Transcribing,
    Complete,
    Error,
    Interrupted,
}

/// Tracks Scribe session lifecycle on disk so incomplete recordings are discoverable after a crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    pub format_version: u8,
    pub state: SessionManifestState,
    pub started_at: String,
    pub mic_wav: String,
    #[serde(default)]
    pub speaker_wavs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Voice-change cuts detected live during recording; populated from the
    /// Transcribing state onward so crash recovery keeps them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speaker_change_cuts: Vec<SpeakerChangeCut>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoverySessionInfo {
    pub session_dir: String,
    pub mic_wav: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictateStateEvent {
    pub state: DictateState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Shares the capture-wide [`ProcessingStage`] vocabulary; Dictate only ever
    /// emits `LoadingModel` and `TranscribingAudio` (no transcript file, no kept audio).
    pub processing_stage: Option<ProcessingStage>,
    /// Populated on Done state — the text that was pasted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Done state only: auto-paste was on but keyboard simulation failed (nothing to paste into,
    /// missing Accessibility permission, etc.). Clipboard still has the text.
    #[serde(default, skip_serializing_if = "crate::types::is_false")]
    pub paste_failed: bool,
    /// Done state only: writing the history entry to disk failed (disk full, bad permissions, etc.).
    /// The transcription was still pasted; the log entry is missing.
    #[serde(default, skip_serializing_if = "crate::types::is_false")]
    pub history_write_failed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Error state only: path to salvaged WAV moved into the user's save folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salvaged_wav_path: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl DictateStateEvent {
    pub fn new(state: DictateState) -> Self {
        Self {
            state,
            progress: None,
            processing_stage: None,
            text: None,
            paste_failed: false,
            history_write_failed: false,
            error: None,
            salvaged_wav_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictateHistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeTranscriptEntry {
    pub path: String,
    pub title: String,
    pub model: String,
    pub modified_at: String,
}

// ── History record store ────────────────────────────────────────────────────────

/// Which capture flow produced a history record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryKind {
    Scribe,
    Dictate,
    Transcribe,
    Written,
}

/// The canonical, source-of-truth record persisted to `{save_folder}/history.jsonl`.
/// One compact JSON object per line. Markdown is a derived, optional output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub format_version: u8,
    pub id: String,
    pub kind: HistoryKind,
    /// RFC3339 UTC timestamp.
    pub created_at: String,
    pub title: String,
    pub model: String,
    /// Raw merged segments with optional channel metadata — re-renderable.
    pub segments: Vec<Segment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speaker_blocks: Vec<SpeakerBlock>,
    /// Voice-change cuts from live pitch/loudness analysis. The full frame
    /// timeline lives in `{session_dir}/analysis.json`, not here (size).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speaker_change_cuts: Vec<SpeakerChangeCut>,
    /// Voice-turn chunks used for chunked Whisper and chunk-level speaker
    /// matching. Empty for legacy records and paths that do not run speaker
    /// analysis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speaker_chunks: Vec<SpeakerChunk>,
    /// Transcript-level speaker centroids derived from clean chunks. Empty for
    /// legacy records and paths that do not run speaker analysis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_speakers: Vec<SessionSpeaker>,
    #[serde(default)]
    pub notes: Vec<Note>,
    pub duration_ms: i64,
    pub word_count: usize,
    #[serde(default)]
    pub speaker_capture: bool,
    #[serde(default)]
    pub dual_source: bool,
    /// Source audio path (Transcribe imports).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Exported markdown path, when written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_path: Option<String>,
    /// Kept-audio session directory (Scribe with keep_wav).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_dir: Option<String>,
    /// Primary kept audio file (e.g. `{session_dir}/mic.wav`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// Markdown text for the `written` Source. None for non-Written records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written_content: Option<String>,
    /// Tombstone: a later line with `deleted = true` removes the record from the live view.
    #[serde(default)]
    pub deleted: bool,
}

/// One completed transcription pass, ready to attach to an existing record.
/// All time-bearing fields are pass-local (t = 0 at the start of the pass);
/// `HistoryRecord::attach_transcript` shifts them into absolute recording time.
#[derive(Debug, Clone, Default)]
pub struct TranscriptAttachment {
    pub segments: Vec<Segment>,
    pub speaker_blocks: Vec<SpeakerBlock>,
    pub speaker_change_cuts: Vec<SpeakerChangeCut>,
    pub speaker_chunks: Vec<SpeakerChunk>,
    pub session_speakers: Vec<SessionSpeaker>,
    pub notes: Vec<Note>,
    pub model: String,
    pub speaker_capture: bool,
    pub dual_source: bool,
    pub session_dir: Option<String>,
    pub audio_path: Option<String>,
    /// None = keep the record's existing markdown path.
    pub markdown_path: Option<String>,
}

/// Where a history list item originated. Legacy sources are read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryItemSource {
    Store,
    LegacyMarkdown,
    LegacyDictate,
}

/// Lightweight projection of a record for the History list UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryListItem {
    pub id: String,
    pub kind: HistoryKind,
    pub created_at: String,
    pub title: String,
    pub model: String,
    pub word_count: usize,
    pub duration_ms: i64,
    pub duration_secs: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub has_markdown: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_path: Option<String>,
    pub source: HistoryItemSource,
}

/// Aggregated dashboard metrics for the home screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub transcript_count: usize,
    pub recorded_this_week_secs: Option<i64>,
    pub float_layers: Option<usize>,
    pub drafts_to_review: Option<usize>,
}

/// One tag name and how many store transcripts reference it (filter panel vocabulary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagVocabularyEntry {
    pub name: String,
    pub count: usize,
}

impl HistoryRecord {
    /// Shared scaffolding: format version, fresh uuid, current UTC time, duration and
    /// word count. `word_count` is computed via the same renderer the `.md` uses so the
    /// store and any exported markdown never diverge.
    fn base(
        kind: HistoryKind,
        title: String,
        model: String,
        segments: Vec<Segment>,
        notes: Vec<Note>,
        word_count: usize,
    ) -> Self {
        let duration_ms = segments.last().map(|s| s.end_ms.max(0)).unwrap_or(0);
        Self {
            format_version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            created_at: chrono::Utc::now().to_rfc3339(),
            title,
            model,
            segments,
            speaker_blocks: Vec::new(),
            speaker_change_cuts: Vec::new(),
            speaker_chunks: Vec::new(),
            session_speakers: Vec::new(),
            notes,
            duration_ms,
            word_count,
            speaker_capture: false,
            dual_source: false,
            source_path: None,
            markdown_path: None,
            session_dir: None,
            audio_path: None,
            written_content: None,
            deleted: false,
        }
    }

    /// Build a Scribe record. `session_dir`/`audio_path` are set when `keep_wav` is on so
    /// the History delete path can remove the kept audio. `markdown_path` is set when the
    /// markdown toggle is on.
    #[allow(clippy::too_many_arguments)]
    pub fn from_scribe(
        title: String,
        model: String,
        segments: Vec<Segment>,
        notes: Vec<Note>,
        speaker_capture: bool,
        dual_source: bool,
        session_dir: Option<String>,
        audio_path: Option<String>,
        markdown_path: Option<String>,
    ) -> Self {
        let word_count = crate::services::output::count_words(&segments);
        let mut rec = Self::base(
            HistoryKind::Scribe,
            title,
            model,
            segments,
            notes,
            word_count,
        );
        rec.speaker_capture = speaker_capture;
        rec.dual_source = dual_source;
        rec.session_dir = session_dir;
        rec.audio_path = audio_path;
        rec.markdown_path = markdown_path;
        rec
    }

    /// Build a Dictate record. Stores the final (post-replacement) dictation text as a single
    /// segment — dictate output is plain text, not a re-renderable transcript. Never has `.md`.
    pub fn from_dictate(segments: &[Segment], text: &str, model: String) -> Self {
        let duration_ms = segments.last().map(|s| s.end_ms.max(0)).unwrap_or(0);
        let word_count = text.split_whitespace().count();
        let title = title_from_text(text);
        let stored = vec![Segment::new(0, duration_ms, text)];
        Self::base(
            HistoryKind::Dictate,
            title,
            model,
            stored,
            Vec::new(),
            word_count,
        )
    }

    /// Build a Transcribe record from an imported audio file.
    #[allow(clippy::too_many_arguments)]
    pub fn from_transcribe(
        title: String,
        model: String,
        segments: Vec<Segment>,
        dual_source: bool,
        source_path: String,
        markdown_path: Option<String>,
    ) -> Self {
        let word_count = crate::services::output::count_words(&segments);
        let mut rec = Self::base(
            HistoryKind::Transcribe,
            title,
            model,
            segments,
            Vec::new(),
            word_count,
        );
        rec.dual_source = dual_source;
        rec.source_path = Some(source_path);
        rec.markdown_path = markdown_path;
        rec
    }

    /// Build a Written note record. Content starts empty — filled in via `update_written_content`.
    pub fn from_written(title: String) -> Self {
        Self::base(
            HistoryKind::Written,
            title,
            String::new(),
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// Attach one transcription pass to this record. The attachment's timelines are
    /// pass-local (t = 0 at the start of the pass); every time-bearing structure is
    /// shifted by this record's current duration so the combined timeline stays in
    /// absolute recording time. Duration and word count are recomputed here — they
    /// are derived fields and must never be shifted or set by callers.
    ///
    /// The destructure is exhaustive on purpose: adding a field to
    /// `TranscriptAttachment` will not compile until this method decides whether
    /// and how it shifts.
    pub fn attach_transcript(&mut self, attachment: TranscriptAttachment) {
        let TranscriptAttachment {
            segments,
            speaker_blocks,
            speaker_change_cuts,
            speaker_chunks,
            session_speakers,
            notes,
            model,
            speaker_capture,
            dual_source,
            session_dir,
            audio_path,
            markdown_path,
        } = attachment;
        let offset_ms = self.duration_ms.max(0);
        let offset_u64 = offset_ms as u64;
        let offset_s = offset_ms as f32 / 1000.0;

        self.segments
            .extend(segments.into_iter().map(|mut segment| {
                segment.start_ms = segment.start_ms.saturating_add(offset_ms);
                segment.end_ms = segment.end_ms.saturating_add(offset_ms);
                segment
            }));
        self.speaker_blocks
            .extend(speaker_blocks.into_iter().map(|mut block| {
                block.start_ms = block.start_ms.map(|ms| ms.saturating_add(offset_u64));
                block.end_ms = block.end_ms.map(|ms| ms.saturating_add(offset_u64));
                block
            }));
        self.speaker_change_cuts
            .extend(speaker_change_cuts.into_iter().map(|mut cut| {
                cut.time_s += offset_s;
                cut.end_s += offset_s;
                cut
            }));
        self.speaker_chunks
            .extend(speaker_chunks.into_iter().map(|mut chunk| {
                chunk.start_ms = chunk.start_ms.saturating_add(offset_u64);
                chunk.end_ms = chunk.end_ms.saturating_add(offset_u64);
                chunk
            }));
        self.session_speakers
            .extend(session_speakers.into_iter().map(|mut speaker| {
                speaker.start_ms = speaker.start_ms.saturating_add(offset_u64);
                speaker.end_ms = speaker.end_ms.saturating_add(offset_u64);
                speaker
            }));
        self.notes.extend(notes.into_iter().map(|mut note| {
            note.recorded_at_ms = note.recorded_at_ms.saturating_add(offset_u64);
            note
        }));

        self.model = model;
        self.speaker_capture = speaker_capture;
        self.dual_source = dual_source;
        self.session_dir = session_dir;
        self.audio_path = audio_path;
        if markdown_path.is_some() {
            self.markdown_path = markdown_path;
        }
        self.duration_ms = self.segments.last().map(|s| s.end_ms.max(0)).unwrap_or(0);
        self.word_count = crate::services::output::count_words(&self.segments);
    }

    /// Project to the lightweight list item shown in History.
    pub fn to_list_item(&self) -> HistoryListItem {
        let excerpt = if self.kind == HistoryKind::Written {
            excerpt_from_written_content(self.written_content.as_deref())
                .or_else(|| excerpt_from_segments(&self.segments))
        } else {
            excerpt_from_segments(&self.segments)
        };
        HistoryListItem {
            id: self.id.clone(),
            kind: self.kind,
            created_at: self.created_at.clone(),
            title: self.title.clone(),
            model: self.model.clone(),
            word_count: self.word_count,
            duration_ms: self.duration_ms,
            duration_secs: self.duration_ms / 1000,
            excerpt,
            tags: Vec::new(),
            has_markdown: self.markdown_path.is_some(),
            markdown_path: self.markdown_path.clone(),
            source: HistoryItemSource::Store,
        }
    }
}

const EXCERPT_MAX_CHARS: usize = 120;

fn excerpt_from_segments(segments: &[Segment]) -> Option<String> {
    let text = segments
        .iter()
        .map(|s| s.text.trim())
        .find(|t| !t.is_empty())?;
    truncate_excerpt(&text.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn excerpt_from_written_content(content: Option<&str>) -> Option<String> {
    let text = content?.trim();
    if text.is_empty() {
        return None;
    }
    truncate_excerpt(text)
}

fn truncate_excerpt(flat: &str) -> Option<String> {
    if flat.is_empty() {
        return None;
    }
    if flat.chars().count() <= EXCERPT_MAX_CHARS {
        Some(flat.to_string())
    } else {
        let truncated: String = flat.chars().take(EXCERPT_MAX_CHARS).collect();
        Some(format!("{truncated}…"))
    }
}

/// Derive a short title from free text: first few words, trimmed, with a sensible fallback.
fn title_from_text(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().take(8).collect();
    let joined = words.join(" ");
    let trimmed = joined.trim_end_matches(|c: char| !c.is_alphanumeric());
    if trimmed.is_empty() {
        "Dictation".to_string()
    } else {
        trimmed.to_string()
    }
}

impl ScribeStateEvent {
    pub fn new(state: ScribeState) -> Self {
        Self {
            state,
            progress: None,
            processing_stage: None,
            transcript_path: None,
            wav_path: None,
            error: None,
            history_record_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub latest_version: String,
    pub current_version: String,
    pub release_url: String,
    pub release_notes: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_with_removed_fields_still_deserializes() {
        // Configs written before the replacement engine and multi-model chooser
        // were removed still carry these keys — serde must ignore them.
        let legacy = r#"{
            "save_folder": "/tmp/transcripts",
            "dictate_model_id": "tiny-en-q5",
            "selected_model_id": "base-en-q5",
            "scribe_model_path": "/old/models/ggml-base.bin",
            "replacement_prefix": "float",
            "replacement_rules": [
                {"trigger": "dash", "type": "simple", "output": "-"}
            ]
        }"#;
        let cfg: Config = serde_json::from_str(legacy).expect("parse legacy config");
        assert_eq!(cfg.save_folder, "/tmp/transcripts");
    }

    #[test]
    fn audio_analysis_serde_roundtrip() {
        let analysis = AudioAnalysis {
            format_version: 1,
            sample_rate: 16_000,
            window_samples: 2048,
            hop_samples: 1024,
            f0_hz: vec![Some(110.0), None, Some(220.0)],
            rms: vec![0.05, 0.001, 0.08],
        };
        let json = serde_json::to_string(&analysis).expect("serialize");
        let parsed: AudioAnalysis = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, analysis);
    }

    #[test]
    fn speaker_change_cut_serializes_lowercase_reasons() {
        let cut = SpeakerChangeCut {
            time_s: 7.5,
            end_s: 7.9,
            score: 1.4,
            reasons: [CutReason::Pitch, CutReason::Loudness]
                .into_iter()
                .collect(),
        };
        let json = serde_json::to_value(&cut).expect("serialize");
        assert_eq!(json["reasons"], serde_json::json!(["pitch", "loudness"]));
        let parsed: SpeakerChangeCut = serde_json::from_value(json).expect("parse");
        assert_eq!(parsed, cut);
    }

    #[test]
    fn session_manifest_without_cuts_field_still_parses() {
        // Manifests written before speaker_change_cuts existed must keep loading.
        let legacy = r#"{
            "format_version": 1,
            "state": "recording",
            "started_at": "2026-05-28T12:00:00Z",
            "mic_wav": "mic.wav"
        }"#;
        let manifest: SessionManifest = serde_json::from_str(legacy).expect("parse legacy");
        assert!(manifest.speaker_change_cuts.is_empty());
        // And empty cuts stay off the wire.
        let json = serde_json::to_value(&manifest).expect("serialize");
        assert!(json.get("speaker_change_cuts").is_none());
    }

    #[test]
    fn history_record_without_cuts_field_still_parses() {
        let legacy = r#"{
            "format_version": 1,
            "id": "abc",
            "kind": "scribe",
            "created_at": "2026-05-28T12:00:00Z",
            "title": "t",
            "model": "m",
            "segments": [],
            "duration_ms": 0,
            "word_count": 0
        }"#;
        let record: HistoryRecord = serde_json::from_str(legacy).expect("parse legacy");
        assert!(record.speaker_change_cuts.is_empty());
        assert!(record.speaker_chunks.is_empty());
        assert!(record.session_speakers.is_empty());
    }

    #[test]
    fn scribe_state_event_serializes_ui_expected_keys() {
        let mut event = ScribeStateEvent::new(ScribeState::Done);
        event.transcript_path = Some("/tmp/result.md".to_string());
        event.progress = Some(0.75);
        event.processing_stage = Some(ProcessingStage::WritingTranscript);

        let json = serde_json::to_value(&event).expect("serialize state event");
        assert_eq!(json["state"], "DONE");
        assert_eq!(json["transcript_path"], "/tmp/result.md");
        assert_eq!(json["progress"], 0.75);
        assert_eq!(json["processing_stage"], "WRITING_TRANSCRIPT");
    }

    #[test]
    fn scribe_state_event_serializes_history_record_id() {
        let mut event = ScribeStateEvent::new(ScribeState::Done);
        event.history_record_id = Some("abc-123".to_string());
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["history_record_id"], "abc-123");
        // Omitted when None (skip_serializing_if)
        let event2 = ScribeStateEvent::new(ScribeState::Done);
        let json2 = serde_json::to_value(&event2).expect("serialize");
        assert!(!json2.as_object().unwrap().contains_key("history_record_id"));
    }

    #[test]
    fn scribe_transcribing_event_carries_progress_lifecycle_field() {
        let mut event = ScribeStateEvent::new(ScribeState::Transcribing);
        event.progress = Some(0.25);
        let json = serde_json::to_value(&event).expect("serialize transcribing event");
        assert_eq!(json["state"], "TRANSCRIBING");
        assert_eq!(json["progress"], 0.25);
    }

    #[test]
    fn dictate_event_keeps_wire_format_with_unified_processing_stage() {
        // Dictate shares ProcessingStage with Record/Upload; the serialized strings
        // must stay what the frontend has always received.
        let mut event = DictateStateEvent::new(DictateState::Transcribing);
        event.processing_stage = Some(ProcessingStage::LoadingModel);
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["processing_stage"], "LOADING_MODEL");
        event.processing_stage = Some(ProcessingStage::TranscribingAudio);
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["processing_stage"], "TRANSCRIBING_AUDIO");
    }

    #[test]
    fn config_save_transcripts_as_markdown_defaults_false_from_old_config() {
        // An old config file missing the new field must still deserialize.
        let old = r#"{"save_folder":"/tmp/x"}"#;
        let cfg: Config = serde_json::from_str(old).expect("deserialize old config");
        assert!(!cfg.save_transcripts_as_markdown);
    }

    #[test]
    fn legacy_history_line_with_embeddings_still_deserializes() {
        // A pre-purge record line carrying biometric fields the types no longer
        // model: they must be ignored, keeping labels and timings readable.
        let old = r#"{"format_version":1,"id":"n1","kind":"scribe","title":"t","created_at":"2026-01-01T00:00:00Z","model":"tiny","segments":[],"duration_ms":1000,"word_count":2,"speaker_chunks":[{"id":"c1","start_ms":0,"end_ms":1000,"label":"Speaker A","cluster_id":"s1","matched_profile":"ben","embedding":[0.1,0.2],"audio_duration_s":1.0,"vad_purity":0.9,"rms_energy":0.1,"clipping":false,"corrections":[{"from_label":"Speaker B","to_label":"Speaker A","corrected_at_ms":5,"auto":false}]}],"session_speakers":[{"session_speaker_id":"s1","label":"Speaker A","centroid_embedding":[0.1],"encrypted_centroid_embedding":{"version":1,"algorithm":"a","nonce_b64":"n","ciphertext_b64":"c"},"clean_chunk_ids":["c1"],"start_ms":0,"end_ms":1000,"duration_ms":1000,"radius":0.1,"quality_score":0.9,"user_confirmed":true}]}"#;
        let rec: HistoryRecord = serde_json::from_str(old).expect("deserialize legacy record");
        assert_eq!(rec.speaker_chunks[0].label, "Speaker A");
        assert_eq!(rec.speaker_chunks[0].corrections.len(), 1);
        assert_eq!(rec.session_speakers[0].label, "Speaker A");
        // Round-trip drops the biometric fields entirely.
        let rewritten = serde_json::to_string(&rec).expect("serialize");
        assert!(!rewritten.contains("embedding"));
        assert!(!rewritten.contains("centroid"));
    }

    #[test]
    fn from_scribe_sets_dual_source_duration_and_kept_audio() {
        let segments = vec![
            Segment {
                start_ms: 0,
                end_ms: 1_000,
                text: "hi".to_string(),
                source: Some(crate::types::SegmentSource::Mic),
            },
            Segment {
                start_ms: 1_200,
                end_ms: 5_000,
                text: "hello there".to_string(),
                source: Some(crate::types::SegmentSource::Speaker),
            },
        ];
        let rec = HistoryRecord::from_scribe(
            "Meeting".to_string(),
            "tiny".to_string(),
            segments,
            vec![],
            true,
            true,
            Some("/save/2026/sess".to_string()),
            Some("/save/2026/sess/mic.wav".to_string()),
            None,
        );
        assert_eq!(rec.kind, HistoryKind::Scribe);
        assert_eq!(rec.duration_ms, 5_000);
        assert!(rec.dual_source);
        assert!(rec.speaker_capture);
        assert_eq!(rec.session_dir.as_deref(), Some("/save/2026/sess"));
        assert!(!rec.id.is_empty());
        assert_eq!(rec.format_version, 1);
    }

    #[test]
    fn from_scribe_can_distinguish_speaker_capture_enabled_from_dual_source_success() {
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 2_000,
            text: "mic only".to_string(),
            source: None,
        }];
        let rec = HistoryRecord::from_scribe(
            "Call".to_string(),
            "tiny".to_string(),
            segments,
            vec![],
            true,
            false,
            None,
            None,
            None,
        );
        assert!(rec.speaker_capture);
        assert!(!rec.dual_source);
    }

    #[test]
    fn from_dictate_word_count_matches_text() {
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 2_000,
            text: "raw".to_string(),
            source: None,
        }];
        let rec = HistoryRecord::from_dictate(&segments, "hello there friend", "tiny".to_string());
        assert_eq!(rec.kind, HistoryKind::Dictate);
        assert_eq!(rec.word_count, 3);
        assert_eq!(rec.duration_ms, 2_000);
        assert_eq!(rec.segments[0].text, "hello there friend");
        assert!(rec.markdown_path.is_none());
    }

    #[test]
    fn from_transcribe_sets_source_path() {
        let segments = vec![Segment {
            start_ms: 0,
            end_ms: 3_000,
            text: "one two".to_string(),
            source: None,
        }];
        let rec = HistoryRecord::from_transcribe(
            "clip".to_string(),
            "tiny".to_string(),
            segments,
            false,
            "/in/clip.mp3".to_string(),
            None,
        );
        assert_eq!(rec.kind, HistoryKind::Transcribe);
        assert_eq!(rec.source_path.as_deref(), Some("/in/clip.mp3"));
        assert_eq!(rec.word_count, 2);
    }

    #[test]
    fn written_record_has_correct_kind() {
        let rec = HistoryRecord::from_written("Title".into());
        assert_eq!(rec.kind, HistoryKind::Written);
        assert!(rec.segments.is_empty());
        assert_eq!(rec.model, "");
        assert_eq!(rec.duration_ms, 0);
        assert_eq!(rec.word_count, 0);
        assert!(rec.written_content.is_none());
        assert!(!rec.id.is_empty());
    }

    #[test]
    fn written_record_deserialises_without_written_content_field() {
        let json = r#"{"format_version":1,"id":"abc","kind":"written","created_at":"2026-01-01T00:00:00Z","title":"T","model":"","segments":[],"notes":[],"duration_ms":0,"word_count":0}"#;
        let rec: HistoryRecord = serde_json::from_str(json).expect("deserialise");
        assert_eq!(rec.kind, HistoryKind::Written);
        assert!(rec.written_content.is_none());
    }

    fn full_attachment() -> TranscriptAttachment {
        TranscriptAttachment {
            segments: vec![Segment::new(0, 2_000, "second part")],
            speaker_blocks: vec![SpeakerBlock {
                label: "You".into(),
                start_ms: Some(0),
                end_ms: Some(2_000),
                text: "second part".into(),
                chunk_id: Some("chunk-1".into()),
            }],
            speaker_change_cuts: vec![SpeakerChangeCut {
                time_s: 0.5,
                end_s: 0.5,
                score: 1.5,
                reasons: [CutReason::Pitch].into_iter().collect(),
            }],
            speaker_chunks: vec![SpeakerChunk {
                id: "chunk-1".into(),
                start_ms: 0,
                end_ms: 2_000,
                label: "Speaker A".into(),
                corrections: Vec::new(),
            }],
            session_speakers: vec![SessionSpeaker {
                session_speaker_id: "s-1".into(),
                label: "Speaker A".into(),
                start_ms: 0,
                end_ms: 2_000,
                duration_ms: 2_000,
            }],
            notes: vec![Note {
                id: "n-1".into(),
                text: "marker".into(),
                recorded_at_ms: 250,
            }],
            model: "base".into(),
            speaker_capture: true,
            dual_source: false,
            session_dir: Some("/sess".into()),
            audio_path: Some("/sess/mic.wav".into()),
            markdown_path: None,
        }
    }

    #[test]
    fn attach_transcript_shifts_every_timeline_structure_by_prior_duration() {
        let mut rec = HistoryRecord::from_scribe(
            "t".into(),
            "tiny".into(),
            vec![Segment::new(0, 1_000, "first")],
            vec![],
            false,
            false,
            None,
            None,
            None,
        );
        rec.attach_transcript(full_attachment());

        assert_eq!(rec.segments.len(), 2);
        assert_eq!(rec.segments[1].start_ms, 1_000);
        assert_eq!(rec.segments[1].end_ms, 3_000);
        assert_eq!(rec.speaker_blocks[0].start_ms, Some(1_000));
        assert_eq!(rec.speaker_blocks[0].end_ms, Some(3_000));
        assert!((rec.speaker_change_cuts[0].time_s - 1.5).abs() < 1e-6);
        assert!((rec.speaker_change_cuts[0].end_s - 1.5).abs() < 1e-6);
        assert_eq!(rec.speaker_chunks[0].start_ms, 1_000);
        assert_eq!(rec.speaker_chunks[0].end_ms, 3_000);
        assert_eq!(rec.session_speakers[0].start_ms, 1_000);
        assert_eq!(rec.session_speakers[0].end_ms, 3_000);
        assert_eq!(rec.notes[0].recorded_at_ms, 1_250);
        assert_eq!(rec.duration_ms, 3_000);
        // "first" + "second part"
        assert_eq!(rec.word_count, 3);
        assert_eq!(rec.model, "base");
        assert!(rec.speaker_capture);
        assert_eq!(rec.session_dir.as_deref(), Some("/sess"));
        assert_eq!(rec.audio_path.as_deref(), Some("/sess/mic.wav"));
    }

    #[test]
    fn attach_transcript_to_record_without_audio_applies_no_offset() {
        let mut rec = HistoryRecord::from_written("T".into());
        rec.attach_transcript(full_attachment());
        assert_eq!(rec.segments[0].start_ms, 0);
        assert_eq!(rec.segments[0].end_ms, 2_000);
        assert!((rec.speaker_change_cuts[0].time_s - 0.5).abs() < 1e-6);
        assert_eq!(rec.notes[0].recorded_at_ms, 250);
        assert_eq!(rec.duration_ms, 2_000);
    }

    #[test]
    fn attach_transcript_keeps_existing_markdown_path_unless_replaced() {
        let mut rec = HistoryRecord::from_written("T".into());
        rec.markdown_path = Some("/old.md".into());

        rec.attach_transcript(full_attachment());
        assert_eq!(rec.markdown_path.as_deref(), Some("/old.md"));

        let mut with_md = full_attachment();
        with_md.markdown_path = Some("/new.md".into());
        rec.attach_transcript(with_md);
        assert_eq!(rec.markdown_path.as_deref(), Some("/new.md"));
    }
}
