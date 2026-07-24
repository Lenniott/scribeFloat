---
title: "Triage: Old biometric fields in history.jsonl until compact finishes"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Old biometric fields in history.jsonl until compact finishes" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- Compact routine: `HistoryService::compact` at `src-tauri/src/services/history.rs:276-302`, run at startup (`src-tauri/src/lib.rs:796`). Loads records via `ensure_loaded` (deserializes each line as `HistoryRecord`), filters tombstones/superseded, then re-serializes and renames the file into place — a full rewrite of every surviving line through the current struct model.
- `HistoryRecord` (`src-tauri/src/types.rs:596-648`) is plain `#[derive(Serialize, Deserialize)]`, no unknown-field passthrough — so legacy keys not on the current struct (`embedding`, `centroid_embedding`, `encrypted_centroid_embedding`, `vad_purity`, `rms_energy`, `quality_score`, etc.) are silently dropped on deserialize and never written back out.
- **Compaction already strips old biometric/voiceprint fields — this is the intended, already-implemented, already-tested behavior** of the 2026-07-16 voiceprint-purge migration (comment at `types.rs:256/280` confirms intent).
- Test coverage exists: `legacy_history_line_with_embeddings_still_deserializes` (`types.rs:1151-1164`) asserts a legacy line with embedding fields deserializes fine and round-trips without those keys; `compact_drops_tombstones_and_superseded` (`history.rs:750`) covers the tombstone/dedup side separately.
- Size estimate: none — no fix required, the described purge is already done and tested.
