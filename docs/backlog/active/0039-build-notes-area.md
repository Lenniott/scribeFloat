---
id: "0039"
title: Build Notes Area screen — list + detail
status: active
---

# Build Notes Area (notes.svelte)

The Notes Area shows all Notes, filterable by source kind and tags.

## Acceptance criteria

- File: `src/lib/screens/notes.svelte`
- List view: filter tabs (All / Scribe / Dictate / Upload), filter side panel (tags), `NoteListCard` per item
- Detail view: `HistoryDetailPane` (existing component — do not rename yet, story 0028 covers that)
- `NoteListCard`: title button opens detail, action icons (copy, open markdown, delete) as siblings with stopPropagation
- Filter panel shows tag vocabulary; empty state says "No vocabulary yet — approve a Float result to populate this layer"
- Prev/next navigation in detail header

## Reference

Exploration: `src/lib/screens/transcripts.svelte` on `release/0.3`.
Rename: `TranscriptListCard` → `NoteListCard`, folder `transcripts/` → `notes/`, copy "Transcripts" → "Notes".
