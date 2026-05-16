use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_save_folder")]
    pub save_folder: String,

    /// Absolute path to the ggml .bin model file used for Scribe.
    /// None = no model configured → NO_MODEL state after recording.
    #[serde(default)]
    pub scribe_model_path: Option<String>,

    #[serde(default)]
    pub keep_wav: bool,

    #[serde(default = "default_true")]
    pub include_timestamps: bool,

    /// Active model id selected by user in model setup.
    #[serde(default)]
    pub selected_model_id: Option<String>,

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

    /// Whisper model ID for Dictate. None = fall back to selected_model_id.
    #[serde(default)]
    pub dictate_model_id: Option<String>,

    /// Simulate Cmd/Ctrl+V into the focused input after dictation.
    /// Requires Accessibility permission on macOS.
    #[serde(default = "default_true")]
    pub dictate_auto_paste: bool,

    /// Simulate Enter after paste (useful for chat/message apps). Default off.
    #[serde(default)]
    pub dictate_auto_enter: bool,

    #[serde(default = "default_replacement_rules")]
    pub replacement_rules: Vec<ReplacementRule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_folder: default_save_folder(),
            scribe_model_path: None,
            keep_wav: false,
            include_timestamps: true,
            selected_model_id: None,
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
            dictate_model_id: None,
            dictate_auto_paste: true,
            dictate_auto_enter: false,
            replacement_rules: default_replacement_rules(),
        }
    }
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

// ── Text replacement types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReplacementRuleType {
    Simple,
    Newline,
    Wrap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReplacementScope {
    Transcripts,
    Dictate,
    #[default]
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WordTransform {
    #[default]
    None,
    Lower,
    Upper,
    Sentence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementRule {
    pub trigger: String,
    /// Additional spoken forms that fire the same rule (e.g. "closed bracket" alongside "close bracket").
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(rename = "type")]
    pub rule_type: ReplacementRuleType,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub scope: ReplacementScope,
    /// wrap type only: text prepended to the following word
    #[serde(default)]
    pub prefix: String,
    /// wrap type only: text appended to the following word
    #[serde(default)]
    pub suffix: String,
    /// wrap type only: case transform applied to the following word
    #[serde(default)]
    pub transform: WordTransform,
}

fn default_replacement_rules() -> Vec<ReplacementRule> {
    vec![
        ReplacementRule {
            trigger: "to do".to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Simple,
            output: "[ ]".to_string(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        },
        ReplacementRule {
            trigger: "open bracket".to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Simple,
            output: "[".to_string(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        },
        ReplacementRule {
            trigger: "close bracket".to_string(),
            aliases: vec!["closed bracket".to_string()],
            rule_type: ReplacementRuleType::Simple,
            output: "]".to_string(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        },
        ReplacementRule {
            trigger: "dash".to_string(),
            aliases: vec![],
            rule_type: ReplacementRuleType::Simple,
            output: "-".to_string(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        },
        ReplacementRule {
            trigger: "new line".to_string(),
            aliases: vec!["newline".to_string()],
            rule_type: ReplacementRuleType::Newline,
            output: String::new(),
            scope: ReplacementScope::Both,
            prefix: String::new(),
            suffix: String::new(),
            transform: WordTransform::None,
        },
    ]
}

fn default_true() -> bool {
    true
}

fn default_save_folder() -> String {
    std::env::var("HOME")
        .map(|h| format!("{}/Documents/Liscribe", h))
        .unwrap_or_else(|_| "/tmp/liscribe".to_string())
}

fn default_open_scribe_hotkey() -> String {
    "CmdOrCtrl+Shift+L".to_string()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub recorded_at_ms: u64,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessingStage {
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

/// Emitted on `model://download-progress` while the default model downloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadEvent {
    pub model_id: String,
    pub progress: f32,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListItem {
    pub id: String,
    pub label: String,
    pub file_name: String,
    pub downloaded: bool,
    pub selected: bool,
    pub size_mb: u32,
    pub wer: f32,
    pub rtfx: Option<u32>,
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DictateProcessingStage {
    LoadingModel,
    TranscribingAudio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictateStateEvent {
    pub state: DictateState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_stage: Option<DictateProcessingStage>,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictateHistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub text: String,
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
    fn scribe_transcribing_event_carries_progress_lifecycle_field() {
        let mut event = ScribeStateEvent::new(ScribeState::Transcribing);
        event.progress = Some(0.25);
        let json = serde_json::to_value(&event).expect("serialize transcribing event");
        assert_eq!(json["state"], "TRANSCRIBING");
        assert_eq!(json["progress"], 0.25);
    }
}
