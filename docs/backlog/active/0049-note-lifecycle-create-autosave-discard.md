---
id: "0049"
title: Note lifecycle — create, autosave, discard-if-empty
status: active
adr: ADR-0009
---

# Note lifecycle — create, autosave, discard-if-empty

Wire up the full lifecycle of a Note in the unified editor: immediate creation on open, autosave, and the discard-if-empty rules on navigate-away.

## What to build

**Backend IPC commands (new):**
- `note_create_empty() -> NoteId` — creates a Note record with a timestamp-derived default title, returns the ID. The Note folder (ADR-0007) is created at this point.
- `note_delete(id: NoteId)` — deletes the Note record and its folder. Safe to call on notes with no content.
- `note_is_empty(id: NoteId) -> bool` — returns true if the Note has no title change, no written content, and no transcript.
- `note_has_metadata(id: NoteId) -> bool` — returns true if tags, keywords, or layer items are set.

**Frontend leave-guard (in `note-editor.svelte`):**

```
on navigate-away:
  if recording in progress → keep note, allow navigation
  else if note_is_empty → note_delete, allow navigation  
  else if note_has_metadata and note_is_empty(content only) → show "Discard or keep?" modal
  else → keep note, allow navigation
```

Use the existing `registerLeaveHandler` / `beforeNavigate` pattern from `+layout.svelte`.

**`/notes/new` route:**
- On mount: invoke `note_create_empty`, then `goto('/notes/' + id, { replaceState: true })`
- The redirect uses `replaceState: true` so the back button skips `/notes/new`

**Autosave:**
- Title: autosaved 500 ms after the user stops typing (via `EditableTitle` existing component's change event)
- Written content: autosaved 800 ms after last keystroke (CodeMirror `updateListener`, story 0045)

## Notes

- The "Discard or keep empty note?" modal uses the existing `Modal` primitive
- Depends on 0044 (shell), 0045 (written editor), 0047 (metadata sidebar) — the leave-guard needs to know about all content types
