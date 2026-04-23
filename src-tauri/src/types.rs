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
        }
    }
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
    "CmdOrCtrl+Shift+S".to_string()
}

fn default_dictate_hotkey() -> String {
    "Ctrl".to_string()
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
    pub transcript_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wav_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    pub kind: String,
    pub granted: bool,
    pub can_request: bool,
}

impl ScribeStateEvent {
    pub fn new(state: ScribeState) -> Self {
        Self {
            state,
            progress: None,
            transcript_path: None,
            wav_path: None,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scribe_state_event_serializes_ui_expected_keys() {
        let mut event = ScribeStateEvent::new(ScribeState::Done);
        event.transcript_path = Some("/tmp/result.md".to_string());
        event.progress = Some(0.75);

        let json = serde_json::to_value(&event).expect("serialize state event");
        assert_eq!(json["state"], "DONE");
        assert_eq!(json["transcript_path"], "/tmp/result.md");
        assert_eq!(json["progress"], 0.75);
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
