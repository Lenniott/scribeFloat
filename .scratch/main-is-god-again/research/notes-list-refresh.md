# Research: Notes list refresh pattern

**Date:** 2026-07-23, from the same load-performance review session.

## What happens

`loadNotes()` (`src/lib/stores/appActions.ts:22-32`) invokes the `history_list` IPC command (full list rebuild, including legacy markdown/dictate scans — `controllers/history.rs:163`) and re-sorts the entire array, client-side, on every single `note://item-added` / `note://item-updated` Tauri event (`src/routes/+layout.svelte:169-174`). This is a full refetch-and-resort per event, not a targeted patch — the more notes accumulate and the more events fire in a burst, the more this costs.

## Event payload shapes (checked against current code)

- `note://item-added` — emitted with **no payload** (`()`) from 5 call sites: `commands/history.rs:140, 236`, `controllers/scribe.rs:1099`, `controllers/dictate.rs:936`, `controllers/transcribe.rs:364`.
- `note://item-updated` — carries `{ id: String }` only (`commands/history.rs:9-20`), no note content.
- No existing IPC command returns a single `HistoryListItem` by id. `history_get_detail` (`commands/history.rs:76-83`) returns a full `HistoryRecord`, a different shape. The only thing that returns `HistoryListItem`s is `HistoryController::list()` — the whole list, not one item.

## What it would take to patch in place instead of refetching

- 3 of 5 emit sites (`controllers/scribe.rs:1099`, `dictate.rs:936`, `transcribe.rs:364`) already build a full `HistoryRecord` in scope right before emitting — a `HistoryListItem` could be assembled from that record with no extra lookup. Two of these (`dictate.rs`, `transcribe.rs`) currently discard the id returned by `history.append(...)` — capturing it is a cheap change.
- 2 of 5 emit sites (`commands/history.rs:140, 236`) only have an `id` in scope, no record — patching those would need a new lightweight `get_list_item(id)` helper (doesn't exist yet; would sit on top of `HistoryService::get()` at `services/history.rs:264` plus the same conversion logic used in `controllers/history.rs:192-213`).

## Open decision

Two viable approaches, not yet decided by the human:
1. **Debounce** — coalesce bursts of `item-added`/`item-updated` events into a single `history_list` refetch. Small, frontend-only, low risk. Doesn't reduce the *cost per refetch*, just the *frequency*.
2. **Full patch-in-place** — change all 5 emit sites to carry the actual list-item data (needs the new `get_list_item` helper for 2 of them), then have the frontend splice/update just that one item in `appState.allItems` instead of refetching. Removes the refetch entirely, but touches backend event payload shapes and needs the new helper.

See [[26-notes-list-refresh-strategy]] for the still-open decision.
