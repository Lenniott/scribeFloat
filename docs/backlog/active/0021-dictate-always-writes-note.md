---
id: "0021"
title: Dictate writes a Note to the store unconditionally
status: active
adr: ADR-0001
---

# Dictate always appends a Note

Currently Dictate saves to a temp file and deletes on success; it writes to `history.jsonl` only optionally. Change: always append a Note on Dictate completion, same as Scribe.

## Where

`src-tauri/src/controllers/dictate.rs` — completion path. Route through `HistoryService::append` unconditionally.
