---
id: "0044"
title: Build unified note editor view at /notes/[id]
status: active
adr: ADR-0006
---

# Build unified note editor view at /notes/[id]

Create the core structural shell of the unified note editor. This is the layout component that all other note editor stories (0045–0051) slot into. It supersedes story 0033 (Scribe screen redesign).

The view lives at `src/lib/ui/5_views/note-editor.svelte` and is routed at `/notes/[id]` and `/notes/new`. `/notes/new` should immediately invoke `note_create_empty` on the backend, then redirect to `/notes/[id]` (replace history, not push).

## Layout

```
┌─────────────────────────────────────────────────────┐
│  ← Notes   [EditableTitle]                          │  ← header, shrink-0
├─────────────────────────────────────────────────────┤
│  [Recording chrome strip]  gear ⚙                  │  ← shrink-0 (story 0046)
├────────────────────────────┬────────────────────────┤
│  [Left panel]              │  [Right panel]         │  ← min-h-0 flex-1
│  Written | Transcript tab  │  Metadata sidebar      │
│  (story 0045 / 0048)       │  (story 0047)          │
└────────────────────────────┴────────────────────────┘
```

- Two equal columns (or ~60/40 split — defer to design token)
- Left panel has a tab strip at top to switch between Written editor and Transcript viewer
- Right panel is always the metadata sidebar
- "← Notes" navigates back to `/notes`; apply leave-guard from ADR-0009 (discard-if-empty)

## Notes

- Wire `/notes/new` in the SvelteKit router (`src/routes/notes/new/+page.svelte`)
- Wire `/notes/[id]` (`src/routes/notes/[id]/+page.svelte`)
- The existing `NoteDetailPane` can be deleted once this view is live
- `CaptureView` overlay in `+layout.svelte` is removed once recording is proven working in this view
- Depends on no other note editor stories to scaffold the shell — the panels can render empty/placeholder initially
