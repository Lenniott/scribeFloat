# ADR-0008: CodeMirror for the written source editor

## Status

Accepted

## Context

The unified note editor (ADR-0006) requires a text editing surface for the `written` Source type. The existing `NoteComposer` is a plain `<textarea>` — suitable for the short timestamped annotations it was built for, but not for long-form markdown composition with formatting cues. The editor needs to handle multi-paragraph content, markdown syntax, and keyboard shortcuts without introducing full WYSIWYG complexity.

Alternatives considered:
- **Plain `<textarea>`** — zero dependency, but no syntax highlighting, no markdown shortcuts, no line-height control for headings. Adequate for short input; poor for long-form writing.
- **Tiptap / ProseMirror** — WYSIWYG with rich formatting UI. Hides markdown syntax from the user; requires a serialisation layer back to markdown for storage. Overkill and an abstraction mismatch for a markdown-native app.
- **Milkdown** — similar to Tiptap; same concerns.
- **CodeMirror 6** — code editor adapted for prose. Renders markdown source with syntax highlighting, heading font-size differentiation, and link detection. Stays in the markdown paradigm. Extensible (autocomplete, keybindings, fold). ~350 KB minified — acceptable inside a Tauri webview where the bundle is already loaded locally.

## Decision

We will use **CodeMirror 6** (`@codemirror/...` packages) as the editor for the `written` Source panel in the unified note editor.

The initial integration uses:
- `@codemirror/lang-markdown` for markdown language support and syntax highlighting
- `@codemirror/view` + `@codemirror/state` for the editor core
- The existing app theme tokens applied via a CodeMirror theme extension (no third-party CM theme)

The editor operates in **source mode** — the user sees and types markdown syntax. There is no live preview toggle inside the editor panel; the transcript panel on the other side of the split serves as the "rendered output" anchor for the session.

Content is stored as raw markdown in the Note's `written` Source.

## Consequences

- `@codemirror/...` packages are added to `package.json`; no Rust/Cargo changes are needed for the editor itself
- The `NoteComposer` textarea component is not extended; it remains for its current use in the Dictate onboarding practice step
- Future extensions (autocomplete for tags, vim mode, fold) can be added incrementally without touching the storage layer
- pulldown-cmark (Rust, backend) is the rendering path for markdown → HTML when the app needs to display rendered output (e.g. export preview, transcript rendering); it is not a dependency of the editor itself
- WYSIWYG markdown editing is explicitly out of scope for this editor
