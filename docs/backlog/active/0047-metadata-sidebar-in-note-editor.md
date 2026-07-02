---
id: "0047"
title: Metadata sidebar in unified note editor
status: active
adr: ADR-0006
---

# Metadata sidebar in unified note editor

Implement the right panel of the note editor as an editable metadata sidebar. This is the nudge surface — always visible as the right column, regardless of what the left panel shows.

Depends on: 0044 (shell). Can be built in parallel with 0045 and 0046.

---

## Backend

### 1. Add metadata fields to `HistoryRecord` in `src-tauri/src/types.rs`

```rust
pub struct HistoryRecord {
    // ... existing fields ...

    /// Tags assigned by the user or Float.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Float Layer Item IDs checked for this note.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_item_ids: Vec<String>,
}
```

`#[serde(default)]` ensures old records without these fields deserialise to empty vecs.

### 2. Add update methods to `HistoryService` in `src-tauri/src/services/history.rs`

Follow the exact `set_markdown_path` pattern (lines 121–132) for each:

```rust
pub fn update_tags(&self, save_folder: &str, id: &str, tags: Vec<String>) -> Result<()>
pub fn update_layer_items(&self, save_folder: &str, id: &str, ids: Vec<String>) -> Result<()>
```

Each clones the record, replaces the relevant field, appends the new line, updates the cache.

### 3. Add methods to `HistoryController` in `src-tauri/src/controllers/history.rs`

```rust
pub fn set_tags(&self, id: &str, tags: Vec<String>) -> Result<(), String>
pub fn set_layer_items(&self, id: &str, ids: Vec<String>) -> Result<(), String>
```

All thin delegations to the service using `self.config.get().save_folder`.

### 4. Add IPC commands in `src-tauri/src/commands/history.rs`

```rust
#[tauri::command]
pub fn note_set_tags(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
    tags: Vec<String>,
) -> Result<(), AppError>

#[tauri::command]
pub fn note_set_layer_items(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
    layer_item_ids: Vec<String>,
) -> Result<(), AppError>
```

All validate `id` with `validate_id`, then delegate to controller. Follow `history_delete` pattern.

### 5. Register in `src-tauri/src/lib.rs`

Add both commands to `tauri::generate_handler![]`.

### 6. Tests

In `src-tauri/src/services/history.rs` tests:

- `update_tags_roundtrips` — append a Written record, call `update_tags` with `["alpha", "beta"]`, reload from disk, assert `record.tags == ["alpha", "beta"]`.
- `old_record_without_tags_deserialises_to_empty` — JSON string without `tags`/`layer_item_ids` deserialises with both as empty vecs.

---

## Frontend

### 1. Create `TagInput` component at `src/lib/ui/2_components/controls/TagInput.svelte`

A chip-style multi-value input. Props:

```ts
let { values = $bindable<string[]>([]), placeholder = 'Add…', onchange }: {
  values?: string[];
  placeholder?: string;
  onchange?: (v: string[]) => void;
} = $props();
```

Behaviour:
- Renders each value as a `<Chip label={v} onremove={() => remove(v)} />` (using existing `Chip` component)
- A trailing `<input type="text">` allows typing
- On `Enter` or `,` keydown: trim the current input, add to values if non-empty and not duplicate, clear input, call `onchange`
- On `Backspace` when input is empty: remove last chip
- Tab-completion autocomplete: accepts an optional `suggestions: string[]` prop; render as a `<datalist>` linked to the input

Export from `src/lib/ui/2_components/controls/index.ts`.

### 2. Create `src/lib/ui/4_sections/NoteMetaSidebar.svelte`

Props:
```ts
let { noteId }: { noteId: string } = $props();
```

Structure (follows `FilterPanel` shell pattern):
```
<aside class="flex flex-col h-full min-h-0 border-l border-card">
  <header class="shrink-0 px-4 py-3 border-b border-card">
    <span class="sf-label-sm text-fg-dim uppercase tracking-wide">Metadata</span>
  </header>
  <ScrollBody>
    <!-- Tags section -->
    <!-- Float Layer Items section -->
  </ScrollBody>
</aside>
```

**Tags section:**
```svelte
<section class="px-4 py-3 border-b border-card">
  <p class="sf-label-sm text-fg-dim mb-2">Tags</p>
  <TagInput
    bind:values={tags}
    suggestions={tagVocabulary}
    onchange={(v) => saveField('tags', v)}
  />
</section>
```

**Float Layer Items section:**
- Load available Layer Items from `invoke<LayerVocabulary>('float_get_vocabulary')` (existing or add if needed)
- Render as a checklist of `<label><input type="checkbox"> {item.label}</label>` items, grouped by Layer
- `onchange` on each checkbox calls `note_set_layer_items` with the full updated set of checked IDs
- If no Layers defined: render `<p class="sf-label-sm text-fg-dim">No Layers defined yet.</p>`

**Data loading on mount:**
```ts
onMount(async () => {
  const record = await invoke<HistoryRecord>('history_get_detail', { id: noteId });
  tags = record.tags ?? [];
  selectedLayerItemIds = record.layer_item_ids ?? [];
  tagVocabulary = await invoke<string[]>('history_tag_vocabulary').then(v => v.map(e => e.value));
});
```

**Autosave helper:**
```ts
async function saveTags(v: string[]) {
  await invoke('note_set_tags', { id: noteId, tags: v });
}
```

### 3. Mount in `note-editor.svelte`

In the right panel slot:
```svelte
<NoteMetaSidebar noteId={id} />
```

### 4. Export from `src/lib/ui/4_sections/index.ts`

Add: `export { default as NoteMetaSidebar } from './NoteMetaSidebar.svelte';`

---

## SOLID / DRY expectations

- `NoteMetaSidebar` = display + IPC only. No business logic.
- `TagInput` is a reusable primitive — no note-specific code inside it.
- All three service methods follow the identical `set_markdown_path` pattern — zero duplication of the JSONL append logic.

---

## Definition of done

- `cargo test -p scribefloat` passes (including new service tests)
- `cargo clippy -- -D warnings` passes
- Tags can be added by typing and pressing Enter; they render as chips
- Tags can be removed by clicking the chip's remove button
- Tags autosave on change (verify by reopening note)
- Float Layer Items checklist renders when layers exist; shows empty state otherwise
- Old notes (without `tags`/`layer_item_ids`) load without error
