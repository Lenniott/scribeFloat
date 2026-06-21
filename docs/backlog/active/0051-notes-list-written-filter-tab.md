---
id: "0051"
title: Add Written tab to Notes list
status: active
adr: ADR-0006
---

# Add Written tab to Notes list

Adds the "Written" filter tab to the Notes list so users can find their typed notes. Purely additive — no new backend needed beyond the `HistoryKind::Written` variant added in story 0044.

Depends on: 0044 (adds `HistoryKind::Written` to the backend). Build after 0044.

---

## Backend

No new IPC commands needed. Verify that `history_list` (via `HistoryController::list_summaries`) includes Written records — it should if `HistoryKind::Written` is serialised as `"written"` by the `#[serde(rename_all = "lowercase")]` on the enum.

Audit `src-tauri/src/controllers/history.rs` in the `list` method for any filter that might exclude Written records. The TODO comments left by story 0044 (`// TODO(0051)`) will flag these locations. Update those arms to include `HistoryKind::Written` in the same branch as `HistoryKind::Scribe` (they share the same list treatment for now).

---

## Frontend

All changes are in `src/lib/ui/5_views/notes.svelte`.

### 1. Extend `CaptureFilter` type

Locate the `CaptureFilter` type (likely `type CaptureFilter = 'all' | 'scribe' | 'dictate' | 'upload'`). Add `'written'`:

```ts
type CaptureFilter = 'all' | 'scribe' | 'dictate' | 'upload' | 'written';
```

### 2. Add tab to `tabs` array

```ts
const tabs: { id: CaptureFilter; label: string }[] = [
  { id: 'all', label: 'All' },
  { id: 'scribe', label: 'Scribe' },
  { id: 'dictate', label: 'Dictate' },
  { id: 'upload', label: 'Upload' },
  { id: 'written', label: 'Written' },  // ← add
];
```

### 3. Add filter logic

In the `filteredItems` derived expression, add the `'written'` case alongside the existing cases:

```ts
case 'written':
  return allItems.filter((item) => item.kind === 'written');
```

### 4. Add `chipForKind` entry

Locate the `chipForKind` map/function and add:

```ts
written: { label: 'Written', variant: 'muted' }
```

(or whatever the existing map structure is — match the pattern of the existing `scribe`, `dictate`, `upload` entries exactly.)

### 5. Update `emptyMessage`

In the `emptyMessage` derived expression, add the `'written'` case:

```ts
case 'written':
  return 'No written notes yet.';
```

### 6. Update description text

Find the string `"Every Scribe, Dictate, and Upload session."` (or similar) in `notes.svelte` and update it to:

```
"Every note — Scribe, Dictate, Upload, and written."
```

---

## SOLID / DRY expectations

- No new components. Pure extension of existing tab/filter pattern.
- Backend audit only — no new service or controller methods.

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- "Written" tab appears in the Notes list tab strip
- Written notes created via the note editor appear in the Written tab
- Written notes also appear in the "All" tab
- The Written tab shows the "No written notes yet." empty state when no written notes exist
- The description text is updated
- Scribe / Dictate / Upload tabs are unaffected
