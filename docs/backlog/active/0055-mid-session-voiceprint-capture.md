---
id: "0055"
title: Mid-session voiceprint capture during Record mode
status: superseeded
adr: ADR-0011
---

# Mid-session voiceprint capture during Record mode

A capture button in the Record mode toolbar lets the user grab a voiceprint of another speaker while a meeting is in progress. The user presses the button while someone else is talking, waits for the quality indicator to reach the target, then names the speaker. The profile is saved and retroactively applied to the full session transcript when transcription runs.

Depends on: 0052 (VoiceprintService), 0056 (transcript renderer uses the labelled output).

---

## Backend

### 1. Mid-session capture state

Add to `ActiveSession` in `src-tauri/src/controllers/scribe.rs`:

```rust
pub struct MidSessionCapture {
    pub buffer: Vec<f32>,       // PCM accumulator for this capture
    pub speech_frames: u32,     // VAD-confirmed speech frames
    pub total_frames: u32,
    pub start_offset_ms: u64,   // position in session when capture started
    pub profile_name: Option<String>,
}
```

### 2. New IPC commands

| Command | Args | Returns |
|---------|------|---------|
| `session_capture_start` | — | `{ capture_id: String }` |
| `session_capture_status` | `capture_id: String` | `CaptureStatus` |
| `session_capture_stop` | `capture_id, profile_name: String` | `CaptureResult` |
| `session_capture_cancel` | `capture_id: String` | `()` |

`CaptureStatus`:
```rust
pub struct CaptureStatus {
    pub speech_s: f32,     // VAD-confirmed speech seconds so far
    pub purity: f32,       // speech_frames / total_frames
    pub state: CaptureState,
}

pub enum CaptureState {
    Pending,    // waiting for speech to start
    Recording,  // speech detected, accumulating
    Safe,       // ≥ 5 s speech reached
    Optimal,    // ≥ 10 s speech reached
}
```

`session_capture_stop` embeds the accumulated buffer and saves the profile to disk. Returns `{ accepted: bool, purity: f32, speech_s: f32 }`.

### 3. Integrate VAD into capture accumulation

During an active `MidSessionCapture`, incoming PCM frames from the mic are:
1. Appended to `capture.buffer`.
2. Passed through the Silero VAD to classify as speech or non-speech.
3. Counted into `speech_frames` / `total_frames`.

Emit a Tauri event `voiceprint://capture-status` every 500 ms with current `CaptureStatus` so the frontend can update the UI without polling.

### 4. Retroactive application

The `session_id` of the current Record session is stored in the capture metadata. When transcription runs (`run_batch`), `VoiceprintService::load_profiles()` picks up all profiles including any captured mid-session. The labelling pass is always over the full session PCM, so retroactive application is automatic — no special code path needed.

### 5. Purity gate

If `purity < 0.5` when `session_capture_stop` is called, the clip is rejected: return `{ accepted: false }`. The frontend shows the Failed state; the user retries.

---

## Frontend

### Capture button

Add to the Record mode toolbar (alongside the existing stop button):

- Icon: microphone with a small plus badge (`mic-add` icon)
- Only visible when a Record session is active
- Tooltip: "Capture speaker voiceprint"

### Capture overlay / popover

When capture is active, show an overlay anchored to the capture button:

```
┌─────────────────────────────────────────┐
│  Capturing voiceprint…                  │
│                                         │
│  [VAD purity bar  ████████░░ 78%]       │
│                                         │
│  [  0s ──── 5s ──────── 10s  ]         │
│      •           ✓ Safe      ★ Optimal  │
│                                         │
│  Duration: 6.2 s  ✓ Safe to stop       │
│                                         │
│  [Cancel]              [Stop & Save]    │
└─────────────────────────────────────────┘
```

State transitions:
- **Pending** (waiting for speech): purity bar empty, counter at 0, "Waiting for speech…"
- **Recording**: purity bar fills, counter increments
- **Safe** (≥ 5 s): counter turns green, "Safe to stop" badge appears
- **Optimal** (≥ 10 s): counter turns gold, "Optimal" badge appears
- **Failed**: red banner "Too noisy — tap Retry", "Retry" and "Cancel" buttons

### Name dialog

After a successful stop, shows an auto-fill name field — same pattern as the enrollment flow (story 0054):

```
┌──────────────────────────────────────────┐
│  Who was that?                           │
│                                          │
│  [ Alice                             ]   │  ← auto-fill: existing profile names
│    ┌─ You                             ┐  │
│    │  Alice                           │  │  ← dropdown of existing profiles
│    └──────────────────────────────────┘  │
│                                          │
│  [Cancel]                    [Save]      │
└──────────────────────────────────────────┘
```

Default text: `"Other"` if no profiles exist, or the most-recently-used profile name. Selecting an existing profile adds this clip to it (improves accuracy); typing a new name creates a profile. On save, the profile is committed and the state icon in the toolbar → ✓ (complete). Icon resets to the idle capture button after 3 s.

### State icon sequence

The capture button icon cycles through 3 states:
1. `mic-add` (idle — ready to capture)
2. `clock` with pulsing ring (active capture in progress)
3. `check-circle` in green (capture saved) — reverts to idle after 3 s

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- Capture button appears only during an active Record session
- Capture accumulates PCM from the mic and emits status events every 500 ms
- Duration counter reaches Safe (green) at 5 s and Optimal (gold) at 10 s
- Purity < 0.5 on stop results in Failed state with retry affordance
- Successful capture saves a profile; profile is visible in settings (story 0057)
- Retroactive labelling: after Record session transcribes, segments before the capture point are also labelled with the new profile
- Cancel at any point discards the buffer with no side effects
- Existing Record start / stop / pause flows are unaffected
