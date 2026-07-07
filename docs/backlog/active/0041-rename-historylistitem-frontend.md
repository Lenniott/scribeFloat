---
id: "0041"
title: Rename HistoryListItem → NoteListItem in frontend types
status: active
adr: ADR-0001
---

# Rename HistoryListItem → NoteListItem

The frontend IPC response type `HistoryListItem` (in `historyActions.ts`) should be `NoteListItem` to match CONTEXT.md.

## Scope

- `src/lib/services/historyActions.ts` — type definition and all exports
- All components importing `HistoryListItem` — update import and type references
- `historyFormat.ts` — update if it references the type directly

## Note

This is purely a TypeScript rename — no Rust changes, no IPC changes. The Rust command still returns the same shape.
