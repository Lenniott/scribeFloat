---
id: "0046"
title: Recording strip in unified note editor
status: active
adr: ADR-0006
---

# Recording strip in unified note editor

Add the persistent recording chrome to the note editor: a horizontal strip above the panels that is always visible regardless of which two panels are showing.

## What to build

**Idle state (compact):** A single "Start Recording" button + gear icon (⚙) to open the settings popover. Strip height ~40px.

**Recording state (expanded):** Waveform (`Waveform` component, existing), `StatusDot`, `RecordingTimer`, "Stop & Save" button, discard `IconButton`. Strip height ~56px.

**Settings popover (gear icon):** Triggered by the gear icon. Contains:
- Mic selector (dropdown, `scribe_list_input_devices`)
- Model selector (dropdown, downloaded models only)
- Speaker capture toggle (`ToggleSwitch`) — can be toggled mid-recording, reuses existing `scribe_toggle_speaker_capture` IPC
- Transcript timestamps toggle (`ToggleSwitch`)
- Input label field and Output label field (text inputs, stored in config, applied at render time)

## Backend wiring

Reuse existing IPC commands: `scribe_start`, `scribe_cancel`, `scribe_toggle_speaker_capture`, `scribe_set_include_timestamps`, `scribe_list_input_devices`. Listen to `scribe://state-changed`, `scribe://audio-level`, `scribe://speaker-level`.

When "Stop & Save" is pressed, the backend processes and attaches the transcript to the current Note via `note_attach_transcript(id: NoteId)` — create this IPC command. The left panel tab automatically switches to the Transcript tab once the transcript is ready (listen for the DONE event).

## Notes

- If recording is active when the user navigates away from the note, the recording continues — show active recording state in TitleBar (already done for Dictate; extend this pattern)
- The strip does not scroll with the panel content — it is `shrink-0` above the panel flex row
- Depends on 0044 (shell)
