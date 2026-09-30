---
title: "Triage: Leaving a note during Record processing drops it from the app"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

New bug found during this effort (not in the original 25-item dump).

Repro: create a new note, press Record, stop, then **leave the note Area while Whisper is still processing**. The capture is written to the on-disk JSON list (`history.jsonl` / session `notes.json`), but the note does not appear in the app (Notes list / Home).

Likely mechanism: a brand-new note is still empty until `note_attach_transcript` runs. The editor leave-guard only skips auto-delete while `recordingActive` is true, and `isRecordingToNote` is `phase === 'recording'` — **not** `'transcribing'`. So navigating away mid-process treats the note as empty and `history_delete`s it. Processing then finishes and writes/attaches against a note the UI has already dropped.

## Question

**Now** or **Later**? Looks like a small Now: the leave-guard already has a "don't delete while capture is in flight" branch, and `appState.scribeAwaitingAttach` is already set for the transcribing wait — it just isn't consulted. Confirm Now and whether the fix is "treat processing like recording (don't delete, allow leave)" vs "block navigation until attach completes."

## Findings

First-pass from the 2026-08-22 report — confirm before building.

- New notes persist immediately via `note_create_empty` (`src-tauri/src/commands/history.rs:137-144`) and emit `note://item-added`. Until a transcript is attached they are empty (`note_is_empty`).
- Record-into-note: `scribe.startRecording(noteId)` sets `scribe_set_attach_note`. On DONE, `ScribeController.handleDone` (`src/lib/stores/scribeController.svelte.ts:229-244`) calls `note_attach_transcript` and sets `transcriptReadyNoteId`. Backend holds `pending_attach` instead of appending a second history row when `attach_note_id` is set (`src-tauri/src/controllers/scribe.rs:1017-1040`). Attach success emits `note://item-added` (`commands/history.rs:229-243`).
- Leave-guard (`src/lib/services/noteLeaveGuard.ts:26-29`) proceeds without delete only if `recordingActive`. Otherwise an empty note is deleted (`history_delete`) and the list refreshes (`onEmptyDeleted` → `loadNotes`).
- `recordingActive` in the editor is `scribe.isRecordingToNote(id)` → `phase === 'recording' && appState.scribeNoteId === noteId` (`scribeController.svelte.ts:79-81`). During processing, `stopAndSave` has already set `phase = 'transcribing'` and `scribeAwaitingAttach = true` (`scribeController.svelte.ts:167-183`). The leave-guard never reads `scribeAwaitingAttach`.
- Tests lock in the gap: `noteLeaveGuard.test.ts` covers "proceeds immediately while recording" and "deletes empty note," but has no transcribing / `scribeAwaitingAttach` case.
- If the empty note is deleted mid-process, attach later fails (`no transcript ready` / missing id) or writes session JSON against a tombstoned row — matches "on disk in the JSON list, not in the app."

**Size estimate: Small.** Extend the leave-guard's in-flight check to cover transcribing (likely `scribeAwaitingAttach` or `phase !== 'idle'`), plus a unit test. Product call: allow leaving (keep the note, attach in the background) vs block until processing finishes.
