---
id: "0066"
title: Persist recording controls in the header across all screens
status: active
---

# Persist recording controls in the header across all screens

As a user recording a meeting, I want to be able to navigate to other parts of the app — my notes list, a previous transcript, settings — without losing the recording or its controls so that I can reference other content mid-session without having to stop.

## The problem

Recording controls (waveform, timer, stop button) currently live inside the note editor's local header via `RecordingStrip.svelte`. They only exist while the user is on `/notes/[id]`. If the user navigates away — to the notes list, settings, or any other route — the note editor unmounts and the strip goes with it. The recording may still be running in the backend, but there is no visible indicator and no way to stop or save without navigating back.

The global `TitleBar` already handles `Dictate` this way — it listens to `dictate://state-changed` and shows a persistent stop button regardless of which screen you're on. Scribe recording has no equivalent.

## What good looks like

When a scribe recording is active, the `TitleBar` shows a compact recording indicator — status dot, timer, stop button — regardless of which route the user is on. The user can freely navigate and the controls follow them. Stopping from any screen triggers transcription and attaches to the note the recording was started from.

The `RecordingStrip` in the note editor can still exist for the idle state (the "Record" button lives there naturally, scoped to the note), but the active recording state should be promoted to the chrome layer.

## Notes

- `TitleBar` already consumes `dictate://state-changed` — add equivalent listener for `scribe://state-changed` and `scribe://audio-level`
- The note association is already stored backend-side via `scribe_set_attach_note` — the TitleBar stop action doesn't need to know which note it's for
- When the user is on the note the recording belongs to, avoid showing duplicate controls — the RecordingStrip and TitleBar should not both show active state at the same time
- Depends on story 0065 (stale capture view removed) to avoid confusion with the old navigation guards
