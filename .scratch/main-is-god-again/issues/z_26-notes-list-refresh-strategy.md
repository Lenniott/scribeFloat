---
title: Decide Notes list refresh strategy (debounce vs patch-in-place)
labels: [wayfinder:grilling]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

`loadNotes()` refetches and re-sorts the *entire* history list on every `note://item-added`/`note://item-updated` event instead of updating just the changed note (full trace: [research/notes-list-refresh.md](../research/notes-list-refresh.md)). Two viable fixes with different cost/risk:

1. **Debounce** — coalesce event bursts into one refetch. Frontend-only, small, doesn't touch backend event shapes.
2. **Full patch-in-place** — carry the actual note data on the events (new `get_list_item` helper needed for 2 of 5 emit sites), splice the one changed item into frontend state instead of refetching.

Which one do we build — or debounce now as a quick win and leave patch-in-place as a later ticket if debounce isn't enough?

## Resolution

**Debounce.** Human confirmed 2026-07-23: coalesce `note://item-added`/`note://item-updated` bursts into a single `history_list` refetch instead of refetching per event. Patch-in-place stays out of scope for now — revisit only if debounce turns out insufficient (e.g. batch-upload-sized bursts of note events become common enough that even one coalesced full-list refetch per burst is noticeable).

Spun off:
- [Implement note-refresh debounce](./issues/z_38-implement-notes-refresh-debounce.md)

## Comments

- 2026-07-23: Raised mid-plan-mode session, deferred when the human redirected to build this as a Wayfinder map instead. Carrying the open question forward here.
- 2026-07-23: Human: "yea same debounce is choice." Decided.
