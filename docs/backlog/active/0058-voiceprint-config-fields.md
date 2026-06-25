---
id: "0058"
title: Voiceprint config fields — user_display_name and voice_similarity_threshold
status: active
adr: ADR-0011
---

# Voiceprint config fields — user_display_name and voice_similarity_threshold

Adds two user-adjustable config fields that control how the voiceprint feature behaves. `user_display_name` controls the label shown for the user's own speech blocks. `voice_similarity_threshold` controls how strict the match gate is. Both are surfaced in Settings → Voice.

Depends on: 0052 (VoiceprintService reads threshold), 0056 (transcript renderer reads `user_display_name`).

---

## Backend

### 1. Add fields to config

In the app config struct (wherever other user settings live — likely `src-tauri/src/services/config.rs` or similar):

```rust
pub struct AppConfig {
    // ... existing fields ...

    /// Display name for the user in speaker-labelled transcripts. Default: "You".
    #[serde(default = "default_user_display_name")]
    pub user_display_name: String,

    /// Cosine similarity gate for voiceprint matching. Default: 0.75.
    /// Range: 0.0–1.0. Lower = more inclusive (may misattribute others).
    /// Higher = more strict (may miss the user's own speech at distance).
    #[serde(default = "default_voice_similarity_threshold")]
    pub voice_similarity_threshold: f32,
}

fn default_user_display_name() -> String { "You".to_string() }
fn default_voice_similarity_threshold() -> f32 { 0.75 }
```

### 2. Expose via IPC

Reuse the existing config get/set IPC pattern. If there is a generic `get_config` / `set_config` command, no new commands are needed — the fields serialize as `user_display_name` and `voice_similarity_threshold` in the config JSON.

If the app uses typed commands per field, add:

| Command | Args | Returns |
|---------|------|---------|
| `set_user_display_name` | `name: String` | `()` |
| `set_voice_similarity_threshold` | `threshold: f32` | `()` |

### 3. Validation

`voice_similarity_threshold` must be clamped to `[0.0, 1.0]`. Reject values outside this range with a descriptive error. `user_display_name` must be non-empty (trim whitespace; if empty after trim, reject).

### 4. Live application

When `voice_similarity_threshold` changes, `VoiceprintService` picks it up on the next `identify()` call — no restart needed (it's just a runtime parameter, not baked into the model).

When `user_display_name` changes, existing transcripts shown in the UI should re-render with the new name (the stored `SpeakerBlock` labels stay as the profile `name`; the frontend maps the user's profile name to `user_display_name` at render time).

---

## Frontend

### Settings → Voice panel (additions to 0057)

Below the enrolled profiles list:

```
  Your display name
  ─────────────────────────────────────────
  [ You                                  ]
  This name appears in your transcripts as [You].


  Speaker matching sensitivity
  ─────────────────────────────────────────
  Strict ◄─────────●──────────────► Inclusive
         0.0      0.75              1.0
  Current: 0.75

  Lower = more of your speech is labelled [You] (may catch some others).
  Higher = only very confident matches are labelled [You] (may miss you at distance).
```

The slider steps in 0.05 increments. Live preview is not required; the setting takes effect on the next transcription.

### Wiring

- On blur of the display name input: call `set_user_display_name`.
- On slider change (debounce 300 ms): call `set_voice_similarity_threshold`.
- Show the current numeric value next to the slider.
- Validate on blur: if name is empty, restore to previous value and show an inline error.

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- `user_display_name` defaults to `"You"` for new installs; persists across restarts
- `voice_similarity_threshold` defaults to `0.75`; persists across restarts
- Changing `user_display_name` is reflected in new transcripts
- Setting threshold to 0.0 labels every segment as the user; setting to 1.0 labels nothing (edge cases handled without panic)
- Invalid values (empty name, threshold out of range) are rejected gracefully
- Existing config fields are unaffected by the migration
