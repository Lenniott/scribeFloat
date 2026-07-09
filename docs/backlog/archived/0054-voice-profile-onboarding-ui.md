---
id: "0054"
title: Voice profile onboarding and enrollment flow
status: done
adr: ADR-0011
---

# Voice profile onboarding and enrollment flow

A first-time onboarding step that walks the user through recording their own voiceprint. The same enrollment UX is used for every profile — "you" and "others" are treated identically. The profile name field is an auto-fill: the user can pick an existing profile (to add more prints to it) or type a new name to create one. After onboarding, the UI explains that more prints make identification more robust and directs the user to Settings → Voice to add more.

Supersedes: story 0053 (auto-enroll from Dictate — dropped; enrollment is explicit and consistent across all profiles).

Depends on: 0052 (VoiceprintService).

---

## Backend

### 1. IPC commands

Add to `src-tauri/src/commands/voiceprint.rs`:

| Command | Args | Returns |
|---------|------|---------|
| `voiceprint_start_clip` | `mic_device_id: String` | `clip_id: String` |
| `voiceprint_stop_clip` | `clip_id: String` | `ClipResult { duration_s, purity, accepted: bool }` |
| `voiceprint_commit_clip` | `clip_id: String, profile_name: String` | `()` |
| `voiceprint_discard_clip` | `clip_id: String` | `()` |
| `voiceprint_list_profile_names` | — | `Vec<String>` |

`voiceprint_start_clip` starts a mic recording. `voiceprint_stop_clip` stops it, runs VAD purity check, embeds the clip, and holds the embedding in memory. `voiceprint_commit_clip` looks up the profile by name (case-insensitive), creates it if new, and calls `VoiceprintService::update_profile_embedding` to merge the clip into the existing profile's rolling mean. `voiceprint_list_profile_names` returns names of all saved profiles for the auto-fill.

### 2. VAD purity check

Same gating as in story 0055: purity < 0.5 → clip rejected; 0.5–0.7 → amber warning; > 0.7 → accepted. Emit `voiceprint://clip-status` events every 500 ms during recording with `{ speech_s, purity, state }`.

### 3. Mic list

Reuse `AudioService::list_input_devices()`.

---

## Frontend

### Onboarding trigger

Show the enrollment onboarding on first launch when no voiceprint profiles exist. Skip if profiles already exist. Can also be re-triggered from Settings → Voice → "Enroll a voice".

### Enrollment flow (same for all profiles)

```
Step 1: Pick a microphone
  ┌──────────────────────────────────────────┐
  │  Which mic should we use?                │
  │  [ Built-in microphone           ▾ ]     │
  │                                  Next → │
  └──────────────────────────────────────────┘

Step 2: Record a clip
  ┌──────────────────────────────────────────┐
  │  Speak naturally for 10 seconds          │
  │  (5 s minimum)                           │
  │                                          │
  │  [VAD purity bar  ████████░░ 82%]        │
  │                                          │
  │  [ 0s ─── 5s ─────── 10s ]              │
  │            ✓ Safe    ★ Optimal           │
  │                                          │
  │  Duration: 7.4 s  ✓ Safe to stop        │
  │                                          │
  │  [Cancel]           [Stop]               │
  └──────────────────────────────────────────┘
  On failure (purity < 0.5): "Too noisy — try again"

Step 3: Name the profile
  ┌──────────────────────────────────────────┐
  │  Who was that?                           │
  │                                          │
  │  [ You                           ]       │  ← auto-fill: existing names + free text
  │    ┌─ You                        ┐       │
  │    │  Alice                      │       │  ← dropdown of existing profiles
  │    └─────────────────────────────┘       │
  │                                          │
  │  Adding to an existing profile makes it  │
  │  more accurate across distances and mics.│
  │                                          │
  │  [Back]              [Save]              │
  └──────────────────────────────────────────┘
```

The name field:
- Shows a dropdown of existing profile names when focused
- Accepts free text to create a new profile
- Default text on first-time onboarding: `"You"`
- Default text on mid-session capture (0055): `"Other"` if no profiles exist, or most-recently-created profile name

### Post-onboarding screen (first-time only)

```
  ✓ Voiceprint saved

  Your transcripts will now label [You] vs [Other].

  More prints = better accuracy.
  Record yourself at different distances or on a different mic
  to make identification more robust.

  You can add more at any time in Settings → Voice.

  [Done]
```

### State

```ts
type EnrollStep =
  | { step: 'pick-mic' }
  | { step: 'recording'; clipId: string; purity: number; speechS: number }
  | { step: 'naming'; clipId: string }
  | { step: 'saving' }
  | { step: 'done'; isFirstTime: boolean }
```

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- Onboarding triggers on first launch when no profiles exist
- User can complete the flow and save a new profile
- Name auto-fill dropdown shows existing profile names; selecting one adds the clip to that profile rather than creating a new one
- Clips with purity < 0.5 are rejected; user can retry without leaving the flow
- Post-onboarding screen shows on first-time enrollment only
- Flow is skipped entirely when profiles already exist (no nag on subsequent launches)
- "Enroll a voice" button in Settings → Voice re-enters the same flow
- Cancel at any step discards the buffered clip with no side effects
