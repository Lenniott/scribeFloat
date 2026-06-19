---
id: "0045"
title: Add CodeMirror written source editor panel
status: active
adr: ADR-0008
---

# Add CodeMirror written source editor panel

Implement the left panel's "Written" tab in the unified note editor as a CodeMirror 6 editor bound to the note's `written` Source.

## What to build

- Add `@codemirror/view`, `@codemirror/state`, `@codemirror/lang-markdown`, and `@codemirror/commands` to `package.json`
- Create `src/lib/ui/2_components/controls/MarkdownEditor.svelte` wrapping a CodeMirror instance
- Style with app design tokens (background = `bg-canvas`, text = `text-fg`, selection = `bg-active/30`) via a CodeMirror theme extension — do not use a third-party CM theme
- Autosave: on every document change, debounce 800 ms, then invoke `note_save_written_source` IPC with the full markdown string
- The editor is the full height of the left panel; no explicit save button

## Backend

`note_save_written_source(id: NoteId, content: String)` — upserts the `written` Source on the Note record. Create this IPC command.

## Notes

- Source mode only — the user sees raw markdown, no WYSIWYG rendering
- Placeholder text: "Start writing…" shown when editor is empty
- The `NoteComposer` textarea is not touched; it remains for the Dictate onboarding step
- Depends on 0044 (shell) for the panel slot
