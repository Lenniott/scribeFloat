---
id: "0046"
title: Recording strip in unified note editor
status: active
adr: ADR-0006
---

# Recording strip in unified note editor

Add the persistent recording chrome to the note editor shell — a horizontal strip above the two panels. Reuses existing `scribe_*` IPC commands and event listeners. Builds on the shell placeholder `div` left by story 0044.

Depends on: 0044 (shell), 0045 (editor, so transcript attachment has a place to show). Build order: 0044 → 0045 → 0046 in parallel with 0047.

---

## Backend

### 1. Add `note_attach_transcript` IPC command in `src-tauri/src/commands/history.rs`

```rust
#[tauri::command]
pub fn note_attach_transcript(
    ctrl: State<'_, Arc<HistoryController>>,
    id: String,
) -> Result<(), AppError>
```

What it does: finds the most recently completed Scribe session (from `ScribeController`'s cached result), calls `ctrl.history.update_segments(&save_folder, &id, segments)` to log-structure-append the transcript segments onto the Written note record.

Add `update_segments` to `HistoryService` following the `set_markdown_path` pattern (lines 121–132 of `src-tauri/src/services/history.rs`): clone record, replace `segments` and recompute `duration_ms`/`word_count`, append line, update cache.

Add `attach_transcript` to `HistoryController`:

```rust
pub fn attach_transcript(&self, id: &str, segments: Vec<Segment>) -> Result<(), String>
```

Thin delegation to `self.history.update_segments(&save_folder, id, segments)`.

IPC command delegates entirely to `ctrl.attach_transcript(&id, segments).map_err(AppError::from)`. Validate `id` with `validate_id`.

### 2. Register in `src-tauri/src/lib.rs`

Add `commands::history::note_attach_transcript` to `tauri::generate_handler![]`.

### 3. Tests

In `src-tauri/src/services/history.rs` tests, add:

- `update_segments_roundtrips` — append a Written record (empty segments), call `update_segments` with 2 segments, reload from disk, assert `record.segments.len() == 2` and `duration_ms > 0`.

---

## Frontend

### 1. Create `src/lib/ui/4_sections/RecordingStrip.svelte`

Props:
```ts
let { noteId }: { noteId: string } = $props();
```

Emits: `ontranscriptready` event when the transcript has been attached and the left panel should switch to the Transcript tab.

**State machine** (Svelte 5 `$state`):
```ts
type StripPhase = 'idle' | 'recording';
let phase = $state<StripPhase>('idle');
let audioLevel = $state(0);
let speakerLevel = $state(0);
let elapsedMs = $state(0);
let settingsOpen = $state(false);
```

**Idle layout** (`min-h-10 flex items-center px-4 gap-3 border-b border-card`):
- "Start Recording" `<button>` with `sf-label-md` class → calls `scribe_start` IPC
- Gear `IconButton` → sets `settingsOpen = true`

**Recording layout** (`min-h-14 flex items-center px-4 gap-3 border-b border-card`):
- `<Waveform>` component (existing, in `src/lib/ui/2_components/`)
- `<StatusDot>` (existing)
- `<RecordingTimer elapsedMs={elapsedMs} />` (existing)
- "Stop & Save" `<button class="sf-label-md">` → calls `scribe_stop` then `note_attach_transcript({ id: noteId })` then emits `ontranscriptready`
- Discard `<IconButton icon="trash">` → calls `scribe_cancel`

**Settings popover** — render as a `<div>` absolutely positioned below the gear icon, `z-50`, card background, when `settingsOpen`. Contents:
- Mic selector: `<select>` populated from `invoke<string[]>('scribe_list_input_devices')`
- Model selector: `<select>` populated from `invoke<ModelInfo[]>('list_models')` filtered to downloaded
- Speaker capture `<Toggle>` — calls `invoke('scribe_toggle_speaker_capture', { enabled })` on change (safe mid-recording)
- Timestamps `<Toggle>` — calls `invoke('scribe_set_include_timestamps', { enabled })` on change
- Input label `<input type="text">` + Output label `<input type="text">` — autosave to config via `invoke('config_update', { ... })`

Close popover on outside click (use `clickoutside` action if available, otherwise `document.addEventListener`).

**Event listeners** (set up in `onMount`, torn down in `onDestroy`):

```ts
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<ScribeStateEvent>('scribe://state-changed', ({ payload }) => {
  phase = payload.state === 'recording' ? 'recording' : 'idle';
});
const unlistenAudio = await listen<number>('scribe://audio-level', ({ payload }) => {
  audioLevel = payload;
});
const unlistenSpeaker = await listen<number>('scribe://speaker-level', ({ payload }) => {
  speakerLevel = payload;
});
```

Elapsed timer: use `setInterval` that increments `elapsedMs` by 100 every 100 ms while `phase === 'recording'`. Clear on `onDestroy`.

**Navigation persistence**: if `phase === 'recording'` when the user navigates away from the note, the recording continues in the background. The existing TitleBar already has an active-recording indicator for Scribe — verify it remains visible and do not suppress it.

### 2. Mount `RecordingStrip` in `note-editor.svelte`

Replace the placeholder `div` from story 0044 with:

```svelte
<RecordingStrip noteId={id} ontranscriptready={() => { activeTab = 'transcript'; }} />
```

Add `activeTab = $state<'written' | 'transcript'>('written')` to note-editor.svelte state.

### 3. Add `RecordingStrip` to `src/lib/ui/4_sections/index.ts`

Export: `export { default as RecordingStrip } from './RecordingStrip.svelte';`

---

## SOLID / DRY expectations

- `RecordingStrip` owns only recording UI state. It does not know about note content.
- `note_attach_transcript` is the single path for associating a transcript with a note — no inline JSONL writes.
- Reuse existing `scribe_*` commands — do not duplicate their logic.

---

## Definition of done

- `cargo test -p scribefloat` passes (including `update_segments_roundtrips`)
- `cargo clippy -- -D warnings` passes
- Clicking "Start Recording" in the note editor starts a recording (waveform + timer appear)
- "Stop & Save" stops recording and attaches the transcript; left panel switches to Transcript tab
- Discard cancels recording and returns strip to idle
- Settings popover opens, speaker capture toggle works mid-recording
- Navigating away while recording leaves the recording active (not cancelled)
