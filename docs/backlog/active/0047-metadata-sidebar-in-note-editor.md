---
id: "0047"
title: Metadata sidebar in unified note editor
status: active
adr: ADR-0006
---

# Metadata sidebar in unified note editor

Implement the right panel of the note editor as an editable metadata sidebar. This is the primary nudge surface — it is always the right panel by default, regardless of what the left panel shows.

## What to build

Create `src/lib/ui/4_sections/NoteMetaSidebar.svelte`. Visual pattern: same shell as `FilterPanel` (header label, `ScrollBody` content, `shrink-0` footer if needed). Not a slide-in overlay — it is a permanent right column within the note editor layout.

**Fields:**

| Field | Input | Backend |
|---|---|---|
| Tags | Multi-value tag input (type + Enter, existing tags autocomplete from vocabulary) | `note_set_tags` |
| Keywords | Same as tags | `note_set_keywords` |
| Float Layer Items | Checklist of available Layer Items per Layer | `note_set_layer_items` |

All fields autosave on change (no debounce needed for discrete selections; tags autosave on Enter/removal).

Load the Note's current metadata on mount via `note_get_metadata(id)` — create this IPC command (or extend `history_get_detail`).

## Notes

- Tags and Keywords inputs: a chip-style multi-value field. Closest existing component is `Chip` (display only) — a new `TagInput` component is likely needed at the component level
- Float Layer Items section only renders if Layers exist in the vocabulary; show empty state ("No Layers defined yet") otherwise
- Depends on 0044 (shell)
- The tag vocabulary for autocomplete reuses `fetchTagVocabulary` from `@services/historyActions`
