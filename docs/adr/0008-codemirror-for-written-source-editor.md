# ADR-0008: CodeMirror for the written source editor

## Status

Accepted

## Context

The unified note editor (ADR-0006) requires a text editing surface for the `written` Source type. The existing `NoteComposer` is a plain `<textarea>` — suitable for the short timestamped annotations it was built for, but not for long-form markdown composition with formatting cues. The editor needs to handle multi-paragraph content, markdown syntax, and keyboard shortcuts without introducing full WYSIWYG complexity.

Alternatives considered:
- **Plain `<textarea>`** — zero dependency, but no syntax highlighting, no markdown shortcuts, no line-height control for headings. Adequate for short input; poor for long-form writing.
- **Tiptap / ProseMirror** — WYSIWYG with rich formatting UI. Hides markdown syntax from the user; requires a serialisation layer back to markdown for storage. Overkill and an abstraction mismatch for a markdown-native app.
- **Milkdown Crepe** — ProseMirror for the main editing surface (CM6 only for code blocks); full WYSIWYG. Same concerns as Tiptap.
- **`@atomic-editor/editor`** — CodeMirror 6 with Obsidian-style inline live preview (markers hidden on non-focused lines). Ships headings, bold, italic, tables as decorations. Ruled out: brand-new solo project, maintenance risk, and the dependency complexity is not justified when CM6 decorations can be added incrementally in-house.
- **CodeMirror 6** — code editor adapted for prose. Stays in the markdown paradigm. Extensible incrementally. ~350 KB minified — acceptable inside a Tauri webview where the bundle is already loaded locally.

## Decision

We will use **CodeMirror 6** (`@codemirror/...` packages) as the editor for the `written` Source panel in the unified note editor.

The initial integration uses:
- `@codemirror/lang-markdown` for markdown language support and GFM extensions (task lists, tables, strikethrough)
- `@codemirror/view` + `@codemirror/state` for the editor core
- `@codemirror/commands` for standard keyboard shortcuts (Mod-B bold, Mod-I italic, etc.)
- The existing app theme tokens applied via a CodeMirror theme extension (no third-party CM theme)

**Markdown styling approach — CSS-first, decorations added incrementally:**

The editor starts with CSS applied to CodeMirror's syntax tokens (`.cmt-heading1`–`.cmt-heading6`, `.cmt-strong`, `.cmt-emphasis`, `.cmt-monospace`, `.cmt-quote`). This gives:
- Headings: larger `font-size` and `font-weight`, `#` markers dimmed to `text-fg-dim`
- Bold/italic: the text between markers is styled; markers stay visible
- Code: monospace font, `bg-fill` background
- Bullets and task list items: `lang-markdown` recognises GFM task lists; visual checkbox rendering is a later addition via CM6 `WidgetDecoration`

Obsidian-style marker-hiding (hide `**` when cursor leaves the line) is **explicitly deferred** — it requires custom `ViewPlugin` work and is not part of the initial build. It can be added per-element type as standalone CM6 extension files without touching the storage or component layers.

Content is stored as raw markdown in the Note's `written` Source.

## Consequences

- `@codemirror/...` packages are added to `package.json`; no Rust/Cargo changes are needed for the editor itself
- The `NoteComposer` textarea component is not extended; it remains for its current use in the Dictate onboarding practice step
- Future extensions (autocomplete for tags, vim mode, fold) can be added incrementally without touching the storage layer
- pulldown-cmark (Rust, backend) is the rendering path for markdown → HTML when the app needs to display rendered output (e.g. export preview, transcript rendering); it is not a dependency of the editor itself
- WYSIWYG markdown editing is explicitly out of scope for this editor
