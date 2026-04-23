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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_folder: default_save_folder(),
            scribe_model_path: None,
            keep_wav: false,
            include_timestamps: true,
            selected_model_id: None,
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
