---
title: Implement debounce for Notes list refresh on item-added/item-updated
labels: [wayfinder:task]
status: closed
assignee: claude-agent
blocked_by: []
parent: MAP.md
---

## Question

`+layout.svelte`'s `onMount` (`src/routes/+layout.svelte:169-174`) currently calls `loadNotes()` directly inside each `note://item-added`/`note://item-updated` listener, causing one full `history_list` refetch + re-sort per event. Per the decided approach ([[26-notes-list-refresh-strategy]]), coalesce bursts of these events into a single refetch.

**Done when:**
1. Rapid-fire `item-added`/`item-updated` events (e.g. a batch of notes created in quick succession) trigger one `loadNotes()` call, not one per event.
2. A single, isolated event still refreshes the list promptly — the debounce window should be short enough (roughly 150-300ms) that a lone note-add doesn't feel delayed.
3. Cleanup on unmount still cancels any pending debounced call (avoid a stray refetch firing after the component is torn down — match the existing cleanup pattern at `+layout.svelte:183-191`).
4. Manual check: create several notes back-to-back (e.g. via rapid Dictate captures or an Upload batch) and confirm the notes list doesn't visibly thrash/re-render per item.
5. Approach recorded in Resolution.

## Resolution

Added a small, reusable `debounce()` helper rather than a one-off inline timer, since `src/lib/utils/` had no existing debounce and this shape (trailing-call coalescing + `cancel()`) is generically useful.

| Cut | Result |
|---|---|
| New utility | `src/lib/utils/debounce.ts` — trailing-edge debounce, generic over args, returns a callable with `.cancel()` |
| Window | 200ms (inside the 150-300ms range) |
| Wiring | `+layout.svelte` onMount wraps both `note://item-added` and `note://item-updated` listeners in one shared `debouncedLoadNotes` instance, so a burst across either/both event types still collapses to one `loadNotes()` call |
| Cleanup | Existing async cleanup (unlisten x3 + `scribe.destroy()`) now also calls `debouncedLoadNotes.cancel()` before awaiting the unlisten promises, so no stray refetch fires post-unmount |
| Barrel export | Added to `src/lib/utils/index.ts`; call site imports directly from `@utils/debounce` to match the existing `@utils/theme` import style in that file |

**Verify:** `npx vitest run` → 120 passed (23 files), including new `src/lib/utils/debounce.test.ts` (4 tests: burst coalesces to one call, single call still fires promptly at the window edge, `cancel()` suppresses a pending call, trailing call receives the latest arguments). `svelte-check` → 0 errors/warnings. `node scripts/check-design-system.mjs` → passed.

Manual check (item 4 in Done-when) not run in this pass — no running app/backend available in this environment to fire real `note://item-added` bursts; the unit tests exercise the debounce logic directly and the wiring was reviewed by hand.

## Comments

- 2026-07-23: Spun off [[26-notes-list-refresh-strategy]] after the human chose debounce over patch-in-place.
- 2026-07-23: Implemented — 200ms trailing debounce via new `src/lib/utils/debounce.ts`, wired into both note listeners in `+layout.svelte`, cleanup cancels pending timer. Closed.
