---
id: "0044"
title: Build unified note editor view at /notes/[id]
status: done
adr: ADR-0006, ADR-0009
---

# Build unified note editor view at /notes/[id]

Creates the route skeleton and the backend foundation (`note_create_empty`) so the app can navigate to a real note. Story 0045 (CodeMirror editor) slots directly into the shell this story produces. These two stories together = MVP "type and save".

---

## Backend — do this first

### 1. Extend `HistoryKind` in `src-tauri/src/types.rs`

```rust
pub enum HistoryKind {
    Scribe,
    Dictate,
    Transcribe,
    Written,   // ← add
}
```

Audit every `match kind` in the codebase (`grep -rn "HistoryKind::" src-tauri/src/`) and add a `Written` arm to each exhaustive match. For list/filter arms that don't yet handle it, map it to the same branch as `Scribe` as a safe default — add a `// TODO(0051)` comment to mark them for the Written filter tab story.

### 2. Add `written_content` to `HistoryRecord` in `src-tauri/src/types.rs`

```rust
pub struct HistoryRecord {
    // ... existing fields ...

    /// Markdown text for the `written` Source. None for non-Written records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written_content: Option<String>,
}
```

`#[serde(default)]` ensures old records without the field deserialise fine.

### 3. Add `HistoryRecord::from_written` constructor in `src-tauri/src/types.rs`

Follow the exact same pattern as the existing `from_scribe`, `from_dictate`, `from_transcribe` constructors in `impl HistoryRecord`. The constructor takes `title: String` and produces a record with `kind: Written`, empty `segments`, empty `notes`, `model: ""`, `duration_ms: 0`, `word_count: 0`, `written_content: None`.

Use `uuid::Uuid::new_v4().to_string()` for the id and `chrono::Utc::now().to_rfc3339()` for `created_at` — same as existing constructors.

### 4. Add `update_written_content` to `HistoryService` in `src-tauri/src/services/history.rs`

Follows the exact same log-structured pattern as `set_markdown_path` (around line 120). Clone the record, set `written_content = Some(content.to_string())`, recompute `word_count` (split on whitespace), append the updated record as a new line, update the in-memory cache.

```rust
pub fn update_written_content(
    &self,
    save_folder: &str,
    id: &str,
    content: &str,
) -> Result<()>
```

### 5. Add methods to `HistoryController` in `src-tauri/src/controllers/history.rs`

```rust
/// Creates a new Written note record and persists it. Returns the new id.
pub fn create_written_note(&self) -> Result<String, String>

/// Updates the written content of an existing note. Content is raw markdown.
pub fn save_written_content(&self, id: &str, content: &str) -> Result<(), String>
```

`create_written_note`: generates a timestamp-derived title (`format!("{}", chrono::Local::now().format("%H:%M %d/%m/%y"))`), calls `HistoryRecord::from_written(title)`, calls `self.history.append(&save_folder, record)`.

`save_written_content`: validates `id` is non-empty (same `validate_id` helper used in `commands/history.rs`), calls `self.history.update_written_content(&save_folder, id, content)`.

### 6. Add IPC commands in `src-tauri/src/commands/history.rs`

```rust
#[tauri::command]
pub fn note_create_empty(
    ctrl: State<'_, Arc<HistoryController>>,
) -> Result<String, AppError>

#[tauri::command]
pub fn note_save_written_content(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
    content: String,
) -> Result<(), AppError>
```

Both are thin wrappers — delegate entirely to the controller. Follow the `history_list` / `history_get_detail` pattern in the same file.

### 7. Register commands in `src-tauri/src/lib.rs`

Add `commands::history::note_create_empty` and `commands::history::note_save_written_content` to the `tauri::generate_handler![]` list alongside the existing `history_*` commands.

### 8. Tests

In `src-tauri/src/types.rs` tests (bottom of file), add:
- `written_record_has_correct_kind` — `HistoryRecord::from_written("Title".into()).kind == HistoryKind::Written`
- `written_record_deserialises_without_written_content_field` — a JSON string without `written_content` deserialises to `written_content: None`

In `src-tauri/src/services/history.rs` tests, add:
- `update_written_content_roundtrips` — append a Written record, call `update_written_content`, reload from disk, assert `written_content == Some(expected)`

---

## Frontend — routes and shell

### Routes

Create `src/routes/notes/new/+page.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';

  onMount(async () => {
    const id = await invoke<string>('note_create_empty');
    await goto(`/notes/${id}`, { replaceState: true });
  });
</script>
```

Create `src/routes/notes/[id]/+page.svelte`:
```svelte
<script lang="ts">
  import { page } from '$app/state';
  import NoteEditorView from '@views/note-editor.svelte';
</script>

<NoteEditorView id={page.params.id} />
```

### Shell view

Create `src/lib/ui/5_views/note-editor.svelte`. Props: `id: string`.

Layout (all existing primitives — no new ones needed for the shell):

```
┌─────────────────────────────────────────────────────┐
│ header: shrink-0, border-b border-card              │
│   "← Notes" button  |  [EditableTitle]              │
├─────────────────────────────────────────────────────┤
│ recording chrome strip: shrink-0 (story 0046)       │
│   placeholder div, min-h-10                         │
├───────────────────────┬─────────────────────────────┤
│ left panel: flex-1    │ right panel: flex-1          │
│ min-h-0 overflow-y    │ min-h-0 overflow-y           │
│ (story 0045 / 0048)   │ (story 0047)                 │
└───────────────────────┴─────────────────────────────┘
```

- Outer: `flex flex-col h-full min-h-0 overflow-hidden bg-panel`
- Panel row: `flex min-h-0 flex-1 overflow-hidden`
- Left and right panels: `flex-1 min-h-0 min-w-0 overflow-y-auto` with a border between them
- "← Notes": `<button onclick={() => goto('/notes')} class="sf-label-md text-fg-dim hover:text-fg">← Notes</button>`
- `EditableTitle` bound to `title`, autosaved 500 ms after change via `note_save_title` (add this command alongside `note_save_written_content` in the backend — same pattern, updates the `title` field on the record)

### Wire `+New Note` in TitleBar

In `src/routes/+layout.svelte`, change `openCapture` to:
```ts
function openCapture() {
  void goto('/notes/new');
}
```
Remove `appState.captureOpen` from the TitleBar `onNewNote` handler. The `CaptureView` overlay stays in the layout for now (don't delete it yet — it guards against regressions while 0046 is unbuilt).

### Leave-guard (stub)

On mount, call `registerLeaveGuard` if provided (same pattern as `scribe.svelte`). For this story: `proceed()` immediately — the full discard-if-empty logic is story 0049.

---

## SOLID / DRY expectations

- **Single Responsibility:** `HistoryService` = persistence only. `HistoryController` = orchestration only. Commands = IPC boundary only. Do not put business logic in commands or persistence logic in the controller.
- **DRY:** `update_written_content` in `HistoryService` is the single path for log-structured content updates — do not inline JSONL appends anywhere else.
- **Open/Closed:** Add `Written` to `HistoryKind` without modifying existing match arms beyond adding the new arm. Do not change existing `from_scribe` / `from_dictate` logic.
- **No dead code:** The `save_written_content` command is used immediately by story 0045 — it is not speculative.

---

## Definition of done

- `cargo test -p scribefloat` passes (including new tests)
- `cargo clippy -- -D warnings` passes
- Clicking `+ New Note` in TitleBar navigates to `/notes/<uuid>` with a visible shell (empty panels, title, back button)
- Back button returns to `/notes`
- The new note record appears in `history_list` with `kind: written`
