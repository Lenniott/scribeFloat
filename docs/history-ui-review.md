# History / Transcripts UI Review

> Read this before changing Transcripts list/detail screens or history components (`transcripts.svelte`, `NoteDetailPane`, list cards, filter panel).

---

## Architecture

Transcripts lives in the main app shell (`transcripts.svelte` route inside `app-shell.svelte`). List and detail are **separate full-height modes** — no `two-column split layout`.

---

## Rules — do not regress

- **List vs detail** are separate full-height modes. Detail opens from the list row title or **View**; list chrome and filter tabs stay hidden until **Close**.
- **Delete** only on list cards for store records; confirm modal on `app-shell.svelte` — cards emit events, never call `history_delete` directly.
- **Detail footer** uses `PanelFooter` (flex `shrink-0` below scroll). Do not add `(deleted FixedFooterBar)` to detail.
- **List card**: title is a `<button>`; action icons are siblings with `stopPropagation` — no nested buttons.
- **Legacy ids** (`md::`, `dictate::`): read-only in UI (no delete/export).
- **Scribe history metadata**: `speaker_capture` = `scribe_capture_speaker` config at write time; `dual_source` = speaker PCM was merged for transcription — do not set both from the same boolean.
- **Layout**: chrome `shrink-0`; one `ScrollBody` body per pane — see ui-enforcement layout-scroll reference.
