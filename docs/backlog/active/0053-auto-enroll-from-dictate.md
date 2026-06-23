---
id: "0053"
title: Auto-enroll user voiceprint from Dictate sessions
status: active
adr: ADR-0011
---

# Auto-enroll user voiceprint from Dictate sessions

Dictate recordings are solo voice — the mic captures only the user. This makes every Dictate session a free enrollment opportunity. After transcription, the full PCM is silently embedded and used to update the user's voiceprint profile for the active mic device. Zero friction; no UI required.

Depends on: 0052 (VoiceprintService). Ships before 0054 so users may already have a working profile when they reach onboarding.

---

## Backend

### 1. Hook into Dictate transcription completion

In `src-tauri/src/controllers/transcribe.rs` (or wherever `run_batch` is called for Dictate sessions), after transcription succeeds:

```rust
// Only for Dictate (quick: true, origin: mic) — not Record or Upload
if note_meta.quick && note_meta.origin == NoteOrigin::Mic {
    let voiceprint_svc = app.state::<VoiceprintService>();
    let mic_id = note_meta.mic_device_id.clone();
    tokio::spawn(async move {
        if let Err(e) = voiceprint_svc.auto_enroll_from_pcm(&pcm, sample_rate, mic_id).await {
            tracing::warn!("auto-enroll failed: {e}");
        }
    });
}
```

Spawn as a background task so it never delays the UI response.

### 2. `auto_enroll_from_pcm` method

Add to `VoiceprintService`:

```rust
pub async fn auto_enroll_from_pcm(
    &self,
    pcm: &[f32],
    sample_rate: u32,
    mic_device_id: Option<String>,
) -> Result<()>
```

Steps:
1. Check if PCM is ≥ 2 s of speech (use existing VAD via `whisper-rs` silence detection or a simple energy check). Skip if too short.
2. Call `self.embed(pcm, sample_rate)` to get a 256-dim embedding.
3. Load existing "You" profile for this `mic_device_id` (or create a new one if absent).
4. Call `self.update_profile_embedding(&mut profile, &embedding)`.
5. Save the updated profile.

### 3. Profile name for user

The auto-enrolled profile always has `name = user_display_name` from config (default `"You"`). If the user later renames themselves in settings (story 0058), the slug stays stable; only `name` is updated.

### 4. Mic device ID

`NoteOrigin` or `NoteMeta` should carry the mic device ID used during recording. If not yet stored, add `mic_device_id: Option<String>` to `HistoryRecord`. Populate it in `MicSession` from `cpal::Device::name()` at recording start.

---

## Frontend

No UI changes. A future enhancement (story 0057) will surface the enrollment count in the profile management screen.

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- After a Dictate session of ≥ 2 s, a `voiceprints/you-builtin-mic.json` (or similar) appears in the data directory
- `sample_count` increments on each subsequent Dictate session
- The background task does not block the Dictate transcription response
- If `auto_enroll_from_pcm` errors (e.g. model not downloaded yet), it logs a warning and returns — it never crashes or surfaces an error to the user
- Existing Dictate flow (quick capture, paste to cursor) is unaffected
