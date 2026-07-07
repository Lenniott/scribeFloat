---
id: "0045"
title: Add CodeMirror written source editor panel
status: done
adr: ADR-0008
---

# Add CodeMirror written source editor panel

Slots the CodeMirror 6 editor into the left panel of the note editor shell produced by story 0044. Together 0044 + 0045 = "type and save" — the MVP written note.

---

## Dependencies

Install via npm (add to `package.json`):

```
@codemirror/view
@codemirror/state
@codemirror/lang-markdown
@codemirror/commands
```

No Rust/Cargo changes for the editor itself.

---

## Backend

### 1. Add `note_save_title` to `HistoryController` in `src-tauri/src/controllers/history.rs`

```rust
/// Updates the title of a note record (log-structured update).
pub fn save_title(&self, id: &str, title: &str) -> Result<(), String>
```

Clone the record, set `title = title.to_string()`, append updated record, update cache. Follow the exact same pattern as `set_markdown_path` in `src-tauri/src/services/history.rs` lines 121–132.

Add a corresponding method to `HistoryService`:

```rust
pub fn update_title(&self, save_folder: &str, id: &str, title: &str) -> Result<()>
```

### 2. Add `note_save_title` IPC command in `src-tauri/src/commands/history.rs`

```rust
#[tauri::command]
pub fn note_save_title(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
    title: String,
) -> Result<(), AppError>
```

Validate `id` with `validate_id`. Delegate to `ctrl.save_title(&id, &title).map_err(AppError::from)`.

### 3. Register in `src-tauri/src/lib.rs`

Add `commands::history::note_save_title` to the `tauri::generate_handler![]` list.

### 4. Tests

In `src-tauri/src/services/history.rs` tests (bottom of file), add:

- `update_title_roundtrips` — append a Written record, call `update_title`, reload from disk, assert `title == "new title"` and the record id is unchanged.

---

## Frontend

### 1. Create `src/lib/ui/2_components/controls/MarkdownEditor.svelte`

Props (Svelte 5 `$props()`):
```ts
let { value = $bindable(''), onchange }: { value?: string; onchange?: (v: string) => void } = $props();
```

The component:
- Creates a `EditorState` with `doc: value`, `extensions: [markdown(), theme, keymap.of(defaultKeymap), EditorView.lineWrapping]`
- Mounts `EditorView` into a `<div bind:this={container}` on `onMount`
- Exposes content changes via `onchange` callback from an `updateListener` extension
- On `value` prop change from outside (e.g. initial load), use `view.dispatch(view.state.update({ changes: { from: 0, to: view.state.doc.length, insert: value } }))` — guard with a dirty flag so roundtrip callbacks don't loop
- Destroys the view in `onDestroy`
- The container `<div>` gets `class="h-full w-full"` — the EditorView fills it

**Theme extension** — define as a `EditorView.theme({})` object using CSS variables from the app design tokens. Do NOT use a third-party CM theme:

```ts
const theme = EditorView.theme({
  '&': { height: '100%', background: 'var(--color-bg-canvas)', color: 'var(--color-fg)' },
  '.cm-content': { fontFamily: 'var(--font-sans)', fontSize: '0.9375rem', padding: '1rem' },
  '.cm-cursor': { borderLeftColor: 'var(--color-fg)' },
  '.cm-selectionBackground, ::selection': { background: 'var(--color-bg-active)' },
  '.cm-gutters': { display: 'none' },
  '.cm-focused': { outline: 'none' },
});
```

**CSS token styling** (add to `src/app.css` or a `<style>` block in the component — prefer global CSS so it applies to the CM shadow DOM):

```css
/* CodeMirror markdown token styles */
.cm-line .cmt-heading1 { font-size: 1.5rem; font-weight: 700; }
.cm-line .cmt-heading2 { font-size: 1.25rem; font-weight: 700; }
.cm-line .cmt-heading3 { font-size: 1.125rem; font-weight: 600; }
.cm-line .cmt-heading4,
.cm-line .cmt-heading5,
.cm-line .cmt-heading6 { font-weight: 600; }
.cmt-heading1, .cmt-heading2, .cmt-heading3 { color: var(--color-fg); }
.cmt-strong { font-weight: 700; }
.cmt-emphasis { font-style: italic; }
.cmt-monospace { font-family: var(--font-mono); background: var(--color-bg-fill); border-radius: 2px; padding: 0 3px; }
.cmt-quote { color: var(--color-fg-dim); border-left: 2px solid var(--color-border-card); padding-left: 0.5rem; }
.cmt-processingInstruction, .cmt-meta { color: var(--color-fg-dim); }
```

**Placeholder** — add the `@codemirror/view` `placeholder` extension:
```ts
import { placeholder } from '@codemirror/view';
// in extensions array:
placeholder('Start writing…')
```

### 2. Wire `MarkdownEditor` into `note-editor.svelte` (from story 0044)

In `src/lib/ui/5_views/note-editor.svelte`:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import MarkdownEditor from '@components/controls/MarkdownEditor.svelte';

  // ... existing props and state ...
  let writtenContent = $state('');
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  function onContentChange(v: string) {
    writtenContent = v;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      await invoke('note_save_written_content', { id, content: v });
    }, 800);
  }
</script>

<!-- inside left panel div: -->
<MarkdownEditor value={writtenContent} onchange={onContentChange} />
```

Load the existing written content on mount: call `invoke<HistoryRecord>('history_get_detail', { id })` and set `writtenContent = record.written_content ?? ''`.

### 3. Wire `EditableTitle` autosave in `note-editor.svelte`

`EditableTitle` already exists at `src/lib/ui/2_components/controls/EditableTitle.svelte`. Bind to `title` state and autosave with 500 ms debounce:

```ts
let titleTimer: ReturnType<typeof setTimeout> | null = null;
function onTitleChange(v: string) {
  title = v;
  if (titleTimer) clearTimeout(titleTimer);
  titleTimer = setTimeout(async () => {
    await invoke('note_save_title', { id, title: v });
  }, 500);
}
```

### 4. Export `MarkdownEditor` from `src/lib/ui/2_components/controls/index.ts`

Add: `export { default as MarkdownEditor } from './MarkdownEditor.svelte';`

---

## SOLID / DRY expectations

- **Single Responsibility:** `MarkdownEditor` owns only the CM instance — no business logic, no IPC calls. All IPC is in `note-editor.svelte`.
- **DRY:** `update_title` in `HistoryService` follows the exact `set_markdown_path` pattern — no duplicated JSONL append logic.
- **No dead code:** Both `note_save_title` and `note_save_written_content` (from 0044) are called by `note-editor.svelte`.

---

## Definition of done

- `cargo test -p scribefloat` passes (including `update_title_roundtrips`)
- `cargo clippy -- -D warnings` passes
- `npm run check` (SvelteKit type check) passes
- Typing in the editor autosaves to the record (verify: open note, type, navigate away, re-open — content persists)
- Heading `#` lines render at a larger font size
- Bold `**text**` renders with bold styling
- Placeholder "Start writing…" appears when editor is empty
- Title edits autosave at 500 ms debounce
