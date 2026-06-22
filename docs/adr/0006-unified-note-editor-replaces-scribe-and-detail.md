# ADR-0006: Unified note editor replaces Scribe panel and NoteDetailPane

## Status

Accepted

## Context

The app has two separate surfaces for note-related work: the Scribe capture panel (recording-first, reached via the TitleBar "New Note" button, overlaying the entire content area) and `NoteDetailPane` (read-only transcript view inside the Notes area). This dual-surface model creates duplication and limits what a Note can be. The domain already defines `written` as a valid Source type but there is no surface to create one. Story 0033 (Scribe screen redesign) flagged the Scribe UI as needing a full rethink; this decision defines the rethink.

## Decision

We will replace both surfaces with a single unified note editor routed at `/notes/[id]`. It handles all four interactions with a Note in one place: creating written content, recording audio, viewing a transcript, and editing Float metadata.

**Layout — three panels, any two visible at once:**
- **Left panel:** Written source editor (CodeMirror) or transcript viewer — switchable via tab control at the top of the left column
- **Right panel:** Metadata sidebar — tags, Float Layer Item assignments — same visual pattern as `FilterPanel`
- **Default pair:** editor + metadata when no transcript exists; transcript + metadata when no written content exists (nudges the user toward metadata entry in both cases)
- **Recording chrome:** persistent strip above the panels — waveform, status dot, elapsed timer, and a gear icon that opens a settings popover (mic selection, model, speaker capture toggle, timestamps toggle)

**Routing:**
- "+ New Note" in the TitleBar navigates to `/notes/new`, immediately creates a Note record on the backend, then redirects to `/notes/[id]`
- Existing notes open directly at `/notes/[id]`
- The `captureOpen` overlay mechanism in `+layout.svelte` is retired for this flow

**What this supersedes:**
- Story 0033 (Scribe screen redesign) — this decision is the redesign
- The current `CaptureView` / `scribe.svelte` panel as the primary capture surface
- `NoteDetailPane` as a read-only view

## Consequences

- A Note with a `written` Source is now a first-class creation flow, not a gap in the product
- `NoteDetailPane` is retired; its responsibilities move into the unified editor (story 0019 deprecation is completed by this work)
- The `captureOpen` / `CaptureView` overlay in `+layout.svelte` becomes dead code and can be removed once the new editor is live
- Recording capability remains fully intact — the recording chrome strip reuses existing backend commands (`scribe_start`, `scribe_cancel`, etc.)
- The Notes list gains a filter tab for `written` notes alongside Scribe, Dictate, and Upload
- Panel-switching keyboard shortcuts are needed (TBD in implementation)
