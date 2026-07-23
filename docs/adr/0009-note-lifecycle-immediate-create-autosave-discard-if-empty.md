# ADR-0009: Note lifecycle — immediate creation, autosave, discard-if-empty

**Status:** Binding
**Wayfinder:** Implemented — Main is God again / current product.

## Context

The unified note editor (ADR-0006) routes to `/notes/[id]`, which means a Note ID must exist before the user has typed anything. The editor also needs a clear answer to: when is content saved, and what happens when the user navigates away from an empty note?

The current Scribe flow creates a Note only at the end of recording (after transcription completes). That model does not work for a writing-first surface where content accumulates incrementally.

## Decision

**Immediate creation:** When the user clicks "+ New Note", the backend creates a Note record immediately and returns its ID. The frontend redirects from `/notes/new` to `/notes/[id]` before the user types anything.

**Autosave:** Written content is saved to the backend on every edit, debounced to ~800 ms of inactivity. There is no explicit "Save" button. The title is also autosaved.

**Discard-if-empty rule:** When the user navigates away from a note, the following rule applies:

| State | Behaviour |
|---|---|
| No title change + no written content + no transcript + no metadata | Note is silently deleted (auto-discard) |
| Metadata set but no content/title/transcript | Prompt: "Discard or keep empty note?" |
| Any content present (written text, title changed, or transcript attached) | Note is kept; autosave has already persisted it |

**Recording exception:** If a recording is in progress when the user navigates away from the note:
- The recording continues in the background (the backend is unaffected)
- The Note is kept regardless of whether written content exists
- The recording chrome in the TitleBar shows active state
- When the user returns to the note (or the recording finishes), the transcript attaches to the same Note

**Title default:** New notes are created with a timestamp-derived default title (`HH:MM DD/MM/YY`). The user can edit it at any time.

## Consequences

- There is no "save" affordance — the mental model is always-saved, like a notes app
- Empty note accumulation is prevented by the auto-discard rule without requiring explicit user action
- The recording-continues-on-leave behaviour allows the user to navigate freely during long recordings without interrupting capture
- The backend needs a `note_create_empty` command that returns an ID, and a `note_delete` command that is safe to call on an empty note
- The `/notes/new` route is a transient redirect, not a persistent state — it should not appear in browser history
- The discard prompt (metadata-but-no-content case) is the only modal in this lifecycle; it follows the existing `Modal` primitive pattern
