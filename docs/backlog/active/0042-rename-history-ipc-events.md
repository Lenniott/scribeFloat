---
id: "0042"
title: Rename history:// and shell:// IPC events to note:// and app://
status: active
adr: ADR-0001
---

# Rename IPC events to match domain model

Current events use stale names that conflict with CONTEXT.md:

| Old | New |
|---|---|
| `history://item-added` | `note://item-added` |
| `shell://navigate` | `app://navigate` |

## Scope

- `src-tauri/src/commands/` — wherever these events are emitted
- `src-tauri/src/controllers/` — same
- All Svelte files that `listen()` to these events
- `app-shell.svelte` (story 0037) should use the new names from day one

## Note

Do this rename in isolation — one commit, easy to verify with a grep.
