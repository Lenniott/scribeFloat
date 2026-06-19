---
id: "0051"
title: Add Written tab to Notes list
status: active
adr: ADR-0006
---

# Add Written tab to Notes list

The Notes list currently filters by Scribe, Dictate, and Upload. Add a "Written" tab for notes created via the unified note editor with a `written` Source.

## What to build

In `src/lib/ui/5_views/notes.svelte`:
- Add `'written'` to the `CaptureFilter` type
- Add `{ id: 'written', label: 'Written' }` to the `tabs` array
- Add filter logic: `allItems.filter((item) => item.kind === 'written')`
- Add `chipForKind` entry: `{ label: 'Written', variant: 'muted' }`
- Update `emptyMessage`: `'No written notes yet.'`

In `notes.svelte` description text, update `"Every Scribe, Dictate, and Upload session."` → `"Every note — Scribe, Dictate, Upload, and written."`

## Backend

Ensure `history_list` (or `note_list`) returns written notes with `kind: 'written'`. The `kind` field is currently set from the source type in `HistoryController::list`. Verify that notes with a `written` Source surface correctly.

## Notes

- Small story — purely additive
- Depends on 0020 (add Source type to Note schema) being complete, so that `kind: 'written'` is available on list items
- Depends on 0044 (note editor) being live so there are actually written notes to show
