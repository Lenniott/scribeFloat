---
id: "0048"
title: Transcript panel in unified note editor
status: active
adr: ADR-0006
---

# Transcript panel in unified note editor

Implement the "Transcript" tab of the left panel in the note editor. This renders the Note's transcript Source as read-only content.

## What to build

A tab strip at the top of the left panel with two tabs: **Written** (story 0045) and **Transcript** (this story). The Transcript tab is hidden/disabled when the Note has no transcript Source.

**Default tab selection:**
- Note has no transcript → Written tab active
- Note has no written content → Transcript tab active
- Note has both → Written tab active (user switches to Transcript manually)

**Transcript rendering:** Invoke `history_render_markdown(id)` (existing IPC) to get the markdown string. Render it as HTML using pulldown-cmark on the backend via a new `note_render_transcript_html(id)` command, or display the raw markdown in a styled read-only CodeMirror instance (no editing). Decision: prefer backend HTML rendering via pulldown-cmark (add `pulldown-cmark` to `src-tauri/Cargo.toml`) — this gives proper heading levels, paragraph spacing, and code blocks without shipping a JS markdown renderer.

Display the rendered HTML inside a `ScrollBody` with prose typography classes.

## Backend

Add `pulldown-cmark = "0.12"` to `src-tauri/Cargo.toml`. Add `note_render_transcript_html(id: NoteId) -> String` IPC command that calls `render_transcript_markdown` (existing) then feeds the result through pulldown-cmark's `html::push_html`.

## Notes

- Transcript is read-only — no editing affordance in this panel
- Copy button in the panel header is desirable (copies full transcript text)
- The transcript panel replaces `NoteDetailPane`'s transcript rendering responsibility
- Depends on 0044 (shell) and 0046 (recording strip, which attaches the transcript to the note)
