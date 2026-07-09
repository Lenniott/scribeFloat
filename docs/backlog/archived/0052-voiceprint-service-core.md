---
id: "0052"
title: VoiceprintService — core speaker embedding and identification
status: done
adr: ADR-0011
---

# VoiceprintService — core speaker embedding and identification

Implements the foundational Rust service that loads the campplus ONNX speaker embedding model via `sherpa-onnx`, maintains enrolled voiceprint profiles on disk, and identifies which profile (if any) a PCM segment belongs to. All downstream stories (0053–0058) depend on this service existing.

Depends on: nothing (new service). Must ship before 0053, 0054, 0055, 0056.

---

## Backend

### 1. Add `sherpa-onnx` dependency

In `src-tauri/Cargo.toml`:

```toml
sherpa-onnx = "1"
```

The crate ships prebuilt ONNX runtime binaries — no additional system dependency.

### 2. Model file

Model: `3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx` (~25 MB).

Download URL:
```
https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx
```

Store at: `~/.scribefloat/models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx`.

Download on first use (not bundled). Verify file presence before initialising extractor; emit a Tauri event `voiceprint://model-downloading` while the download is in progress.

### 3. Profile store

Each voiceprint is a JSON file at `~/.scribefloat/voiceprints/{slug}.json`:

```json
{
  "name": "You",
  "mic_device_id": "AppleHDA:0",
  "embedding": [0.023, -0.411, ...],
  "sample_count": 12,
  "updated_at": "2026-06-23T10:00:00Z"
}
```

`embedding` is the mean L2-normalised vector across all enrolled samples (256 floats). `sample_count` tracks how many clips contributed so rolling-average updates remain weighted correctly.

### 4. `VoiceprintService` struct

Create `src-tauri/src/services/voiceprint.rs`:

```rust
pub struct VoiceprintService {
    extractor: SpeakerEmbeddingExtractor,
    profiles_dir: PathBuf,
    threshold: f32,
}

impl VoiceprintService {
    pub fn new(model_path: &Path, profiles_dir: &Path, threshold: f32) -> Result<Self>
    pub fn embed(&self, pcm: &[f32], sample_rate: u32) -> Result<Vec<f32>>
    pub fn load_profiles(&self) -> Result<Vec<VoiceprintProfile>>
    pub fn save_profile(&self, profile: &VoiceprintProfile) -> Result<()>
    pub fn delete_profile(&self, slug: &str) -> Result<()>
    pub fn identify(&self, embedding: &[f32], profiles: &[VoiceprintProfile]) -> String
    pub fn update_profile_embedding(
        &self, profile: &mut VoiceprintProfile, new_embedding: &[f32]
    )
}
```

`identify` returns the `name` of the closest profile if `max_cosine_sim >= self.threshold`, else `"Other"`.

`update_profile_embedding` implements the rolling mean: `new_mean = (old_mean * sample_count + new_embedding) / (sample_count + 1)`, then L2-normalises.

### 5. `VoiceprintProfile` struct

```rust
pub struct VoiceprintProfile {
    pub name: String,
    pub slug: String,
    pub mic_device_id: Option<String>,
    pub embedding: Vec<f32>,
    pub sample_count: u32,
    pub updated_at: DateTime<Utc>,
}
```

### 6. Register service

Add `VoiceprintService` to the Tauri app state alongside `AudioService` and `ModelService`. Initialise lazily on first use (model download may be pending).

### 7. IPC commands

New commands in `src-tauri/src/commands/voiceprint.rs`:

| Command | Args | Returns |
|---------|------|---------|
| `voiceprint_list_profiles` | — | `Vec<ProfileSummary>` |
| `voiceprint_delete_profile` | `slug: String` | `()` |
| `voiceprint_rename_profile` | `slug: String, name: String` | `()` |
| `voiceprint_model_status` | — | `{ downloaded: bool, path: String }` |
| `voiceprint_download_model` | — | emits progress events, returns `()` |

---

## Frontend

No UI in this story — service and IPC surface only. UI is in stories 0054, 0055, 0057.

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `VoiceprintService::new()` succeeds when model file is present
- `embed()` returns a 256-element vector for a 16 kHz mono PCM slice of ≥ 2 s
- `identify()` returns the correct profile name for a matching embedding and `"Other"` when below threshold
- `save_profile()` and `load_profiles()` round-trip correctly
- `voiceprint_list_profiles` IPC command returns an empty array when no profiles exist
- `voiceprint_model_status` returns `{ downloaded: false }` before download and `{ downloaded: true }` after
