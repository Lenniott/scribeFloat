---
id: "0049"
title: Note lifecycle — create, autosave, discard-if-empty
status: done
adr: ADR-0009
---

# Note lifecycle — create, autosave, discard-if-empty

Completes the note lifecycle: immediate creation, autosave, and discard-if-empty leave-guard.

---

## Done

- [x] Frontend dirty-check autosave (title + written)
- [x] Written body → `.notes/{id}/written.md` (in-place)
- [x] Title → `.notes/{id}/meta.json` (in-place)
- [x] `note_is_empty` / `note_has_metadata` backend + IPC
- [x] Real leave-guard via `appState.noteLeaveGuard` + `runNoteLeaveGuard`
- [x] Discard modal for metadata-only empty notes
- [x] `RecordingStrip` bindable `recordingActive`
- [x] Controller + sidecar + Vitest tests
- [x] `/notes/new` → `/notes/<uuid>` with `replaceState: true`

See `docs/action-flows.md` §6f and `docs/engineering/history-storage.md`.
