---
id: "0023"
title: Add triage status field to Note
status: active
adr: ADR-0004
---

# Add triage status to Note

A Note needs a machine-readable triage status field.

## Values

- `none` — no Float results pending
- `pending` — one or more draft Results
- `triaged` — user has reviewed

This field gates the Triage surface on Home (story 0024).

## Where

`src-tauri/src/types.rs` — add `triage_status: TriageStatus` with `#[serde(default)]` defaulting to `none`.
