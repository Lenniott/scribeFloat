---
id: "0014"
title: Rename HistoryRecord → Note throughout the codebase
status: active
adr: ADR-0001
---

# Rename HistoryRecord → Note

Rename `HistoryRecord` to `Note` everywhere so code speaks the same language as CONTEXT.md and the ADRs.

## Scope

- `src-tauri/src/types.rs` — struct rename
- `services/history.rs` — all usages
- `controllers/` — all usages
- IPC command names
- Svelte event names

## Notes

Behaviour change: none. Pure rename. Do in one commit so the repo is never in a half-renamed state.
