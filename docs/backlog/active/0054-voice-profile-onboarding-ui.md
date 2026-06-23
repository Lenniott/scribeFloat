---
id: "0054"
title: Voice profile onboarding UI
status: active
adr: ADR-0011
---

# Voice profile onboarding UI

A guided enrollment flow that lets the user record their voice across multiple mic / distance combinations. Produces a robust voiceprint that handles variation in how the user is positioned relative to their microphone. Accessed from Settings → Voice → "Enroll my voice".

Depends on: 0052 (VoiceprintService), 0053 (auto-enroll gives a baseline before this runs).

---

## Backend

### 1. New IPC commands

Add to `src-tauri/src/commands/voiceprint.rs`:

| Command | Args | Returns |
|---------|------|---------|
| `voiceprint_start_enrollment_clip` | `mic_device_id: String` | `session_id: String` |
| `voiceprint_stop_enrollment_clip` | `session_id: String` | `EnrollmentClipResult { duration_s, purity_score, accepted: bool }` |
| `voiceprint_commit_enrollment` | `profile_name: String, mic_device_id: String` | `()` |
| `voiceprint_cancel_enrollment` | — | `()` |

`voiceprint_start_enrollment_clip` starts a mic recording using `AudioService::start_mic()`.

`voiceprint_stop_enrollment_clip` stops recording, runs VAD purity check (ratio of speech frames to total), embeds the clip, and holds the embedding in a temporary in-memory buffer (not persisted yet).

`voiceprint_commit_enrollment` averages all buffered embeddings, L2-normalises, and calls `VoiceprintService::save_profile()`.

### 2. VAD purity check

Purity = (speech frames / total frames) where a frame is 20 ms. Use the Silero VAD model already present in the app (`ggml-silero-v6.2.0.bin` via `whisper-rs`). Purity < 0.6 = amber warning; < 0.4 = clip rejected. Emit as part of `EnrollmentClipResult`.

### 3. Mic list

Reuse `AudioService::list_input_devices()` to populate the mic selector in the UI.

---

## Frontend

### Flow structure

A multi-step modal/sheet. Route: `Settings → Voice → Enroll my voice` button.

```
Step 1: Pick a microphone
  - Dropdown of available input devices
  - "Next →"

Step 2–4: Record clips (one per distance)
  - Prompt text: "Sit at your normal working distance and say a few sentences."
                 "Now move a little further back and record again."
                 "One more — this time closer to the mic."
  - [Record] button → starts clip
  - VAD purity bar (green / amber / red) shown during recording
  - Duration counter: 0 s → 5 s (safe, green) → 10 s (optimal, gold)
  - [Stop] button
  - Result: ✓ Accepted  /  ⚠ Noisy — try again  /  ✗ Too short
  - "Retry" or "Next →" depending on result

Step 5: Name and save
  - Text input: "What should we call this profile?" (default: "You")
  - "Save voiceprint" button
  - On success: sheet closes, toast "Voiceprint saved — speaker labels are now active."
```

### State

```ts
type EnrollmentState =
  | { step: 'pick-mic' }
  | { step: 'record'; pass: 1 | 2 | 3; recording: boolean; purity: number; duration_s: number }
  | { step: 'name'; clips_accepted: number }
  | { step: 'saving' }
  | { step: 'done' }
```

### VAD purity bar

Colour thresholds:
- `purity >= 0.8` → green
- `purity >= 0.6` → amber
- `purity < 0.6` → red (clip will be rejected on stop)

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- User can complete the 3-pass enrollment flow and save a profile
- Clips with purity < 0.4 are rejected and require retry
- Saved profile appears in Settings → Voice → Enrolled profiles list (story 0057)
- Cancel at any step discards all buffered clips with no side effects
- Existing settings flows are unaffected
