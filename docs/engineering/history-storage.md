# History storage

> Load when changing how notes are persisted, autosaved, or loaded from disk.

---

## Two tiers

| Tier | Path | When it writes | Update style |
|------|------|--------------|--------------|
| **Capture log** | `{save_folder}/history.jsonl` | New scribe/dictate/upload/written record; attach transcript; export markdown path; delete (tombstone) | Append-only line per event; loader keeps last line per `id` |
| **Editor sidecars** | `{save_folder}/.notes/{id}/` | Title rename; CodeMirror body edits; (future) tags/keywords in `meta.json` | Overwrite in place |

Startup `compact()` collapses `history.jsonl` to one live line per note (drops tombstones and superseded capture lines).

---

## Sidecar layout (interim)

```
{save_folder}/.notes/{uuid}/
  written.md    ← CodeMirror body (overwrite on debounced autosave)
  meta.json     ← `{ "title": "..." }` (+ tags/keywords when 0047 lands)
```

On load, `HistoryService` reads jsonl then **hydrates** sidecars onto the in-memory record (`note_sidecar::hydrate_record`).

Legacy `written_content` / title fields still in jsonl are used when no sidecar exists (pre-migration notes).

Story **0050** (ADR-0007) will replace `.notes/{uuid}/` with named per-note folders and optional `note.md` export — extend `note_sidecar.rs`, do not duplicate paths elsewhere.

---

## What appends to jsonl

- `append` — new capture or empty written note
- `update_segments` — transcript attached to a note
- `remove_voice_embeddings` — biometric vectors removed while transcript text/labels remain
- `set_markdown_path` — export path recorded
- `delete` — tombstone line

## What does **not** append to jsonl

- `update_written_content` → `written.md`
- `update_title` → `meta.json`

Frontend autosave must **dirty-check** before calling IPC (see `note-editor.svelte`).

---

## Delete

`HistoryService::delete` tombstones jsonl and removes `{save_folder}/.notes/{id}/`.

---

## Code map

| Module | Role |
|--------|------|
| `services/history.rs` | jsonl append, compact, cache, orchestrates sidecar calls |
| `services/note_sidecar.rs` | All `.notes/` path I/O and hydration |
| `controllers/history.rs` | IPC-facing orchestration; no direct file writes |

---

## Tests

- `services/history.rs`: `update_*_does_not_append_jsonl_lines`, roundtrips after reload
- `services/note_sidecar.rs`: meta merge on write
