---
id: "0049"
title: Note lifecycle — create, autosave, discard-if-empty
status: active
adr: ADR-0009
---

# Note lifecycle — create, autosave, discard-if-empty

Completes the note lifecycle: immediate creation, autosave (already wired in 0044/0045), and the discard-if-empty leave-guard. Story 0044 provides the stub `proceed()` leave-guard; this story replaces it with the real logic.

Depends on: 0044 (shell + create/save commands), 0045 (written editor), 0047 (metadata sidebar). Build after all three.

---

## Backend

### 1. Add `note_is_empty` and `note_has_metadata` to `HistoryController` in `src-tauri/src/controllers/history.rs`

```rust
/// True if the note has no written content, no transcript segments, and an unmodified (default) title.
pub fn is_empty(&self, id: &str) -> Result<bool, String>

/// True if tags, keywords, or layer_item_ids are non-empty.
pub fn has_metadata(&self, id: &str) -> Result<bool, String>
```

Both call `self.history.get(&save_folder, id)` and inspect the record fields. A "default title" is one that matches the `chrono::Local::now().format("%H:%M %d/%m/%y")` pattern from `create_written_note` — compare against the record's `created_at` to regenerate the expected default title, or simply check if `title` is empty or matches the timestamp format `HH:MM DD/MM/YY` via a regex.

Simpler heuristic accepted: treat title as "unmodified" if it equals `format!("{}", chrono::DateTime::parse_from_rfc3339(&record.created_at)?.with_timezone(&chrono::Local).format("%H:%M %d/%m/%y"))`.

### 2. Add IPC commands in `src-tauri/src/commands/history.rs`

```rust
#[tauri::command]
pub fn note_is_empty(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<bool, AppError>

#[tauri::command]
pub fn note_has_metadata(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<bool, AppError>
```

Both validate `id` with `validate_id`, delegate to controller.

> `note_delete` was already registered in story 0044 as it delegates to the existing `HistoryController::delete` method. Verify it is present; if not, add the IPC command now following the same pattern as `history_delete`.

### 3. Register in `src-tauri/src/lib.rs`

Add `commands::history::note_is_empty` and `commands::history::note_has_metadata` to `tauri::generate_handler![]`.

### 4. Tests

In `src-tauri/src/controllers/history.rs` tests:

- `is_empty_returns_true_for_fresh_note` — create a Written record with `from_written`, append it, call `is_empty` → expect `true`.
- `is_empty_returns_false_after_content_added` — append Written record, call `update_written_content` with `"hello"`, call `is_empty` → expect `false`.
- `has_metadata_returns_false_for_fresh_note` — create Written record, call `has_metadata` → expect `false`.
- `has_metadata_returns_true_after_tags_set` — append Written record, call `update_tags` with `["tag1"]`, call `has_metadata` → expect `true`.

---

## Frontend

### 1. Replace the stub leave-guard in `note-editor.svelte`

Story 0044 installs `registerLeaveGuard(() => proceed())`. Replace that stub with the real guard:

```ts
import { registerLeaveGuard } from '$lib/navigation'; // existing pattern from scribe.svelte

registerLeaveGuard(async (proceed, cancel) => {
  if (recordingActive) {
    // Recording continues in background — just navigate away
    proceed();
    return;
  }
  const empty = await invoke<boolean>('note_is_empty', { id });
  if (empty) {
    await invoke('history_delete', { id });
    proceed();
    return;
  }
  const hasMeta = await invoke<boolean>('note_has_metadata', { id });
  if (hasMeta) {
    // Show "Discard or keep empty note?" prompt only if metadata but no content
    // (content-only empty is already handled above since empty=true when no content AND no transcript)
    // If is_empty=false (has content or transcript) we don't reach here
    // This branch triggers when: metadata is set but written_content and segments are both empty
    showDiscardModal = true;
    // Do not call proceed() or cancel() yet — wait for modal response
    pendingProceed = proceed;
    pendingCancel = cancel;
    return;
  }
  proceed();
});
```

Add `showDiscardModal = $state(false)` and handlers:

```ts
async function onDiscard() {
  showDiscardModal = false;
  await invoke('history_delete', { id });
  pendingProceed?.();
}
function onKeep() {
  showDiscardModal = false;
  pendingCancel?.();
}
```

### 2. Add `Modal` for discard prompt in `note-editor.svelte`

Use the existing `Modal` primitive from `src/lib/ui/1_primitives/`:

```svelte
{#if showDiscardModal}
  <Modal title="Discard empty note?">
    <p class="sf-body-sm text-fg-dim">
      This note has metadata but no content. Discard it or keep it as an empty note?
    </p>
    <svelte:fragment slot="actions">
      <button class="sf-label-md text-danger" onclick={onDiscard}>Discard</button>
      <button class="sf-label-md" onclick={onKeep}>Keep</button>
    </svelte:fragment>
  </Modal>
{/if}
```

Check the existing `Modal` component's props/slots in `src/lib/ui/1_primitives/Modal.svelte` and match its API exactly.

### 3. `recordingActive` state

`RecordingStrip` (story 0046) owns recording state. Pass it up via a bindable prop or a Svelte 5 snippet callback:

```svelte
<!-- in note-editor.svelte -->
let recordingActive = $state(false);
<RecordingStrip noteId={id} bind:recordingActive ... />
```

In `RecordingStrip.svelte`, add to props: `let { ..., recordingActive = $bindable(false) } = $props();` and set `recordingActive = phase === 'recording'` whenever `phase` changes.

---

## SOLID / DRY expectations

- `is_empty` and `has_metadata` are pure queries on the record — no side effects, no file I/O beyond the existing cached read.
- The delete path (`history_delete` IPC, already exists) is the single note deletion path — do not add a second.
- The discard modal uses the existing `Modal` primitive — do not build a new one.

---

## Definition of done

- `cargo test -p scribefloat` passes (including new controller tests)
- `cargo clippy -- -D warnings` passes
- Opening a note, typing nothing, and navigating away → note is silently deleted from `history_list`
- Opening a note, typing content, and navigating away → note persists in `history_list`
- Opening a note, adding metadata only, and navigating away → "Discard or keep?" modal appears
- While recording: navigating away does not cancel the recording and does not delete the note
- `/notes/new` route redirects to `/notes/<uuid>` with `replaceState: true` (browser back button skips it)
