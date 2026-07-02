---
id: "0020"
title: Add Source type to Note/HistoryRecord schema
status: active
adr: ADR-0002
---

# Add Source type to Note schema

Each Note needs a `sources` array where each entry has a `type` and its content.

## Source types

`transcript | written | upload_audio | web | video | import_md`

## Scope for this story

Start with `transcript` and `written` — the only two that exist today. The others are placeholders for future flows.

## Where

`src-tauri/src/types.rs` — add `sources: Vec<NoteSource>` to the Note/HistoryRecord struct with `#[serde(default)]`.
