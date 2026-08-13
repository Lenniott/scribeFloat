---
title: "Triage: Old biometric fields in history.jsonl until compact finishes"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

Ticket claims stale biometric/voiceprint fields (`embedding`, `centroid_embedding`, `rms_energy`, etc.) linger in `history.jsonl` until compaction runs. Investigation confirms `HistoryService::compact` already silently drops these fields on every rewrite, as intended by the 2026-07-16 voiceprint-purge migration, and this behavior is already covered by tests.

## Question

Read the "Old biometric fields in history.jsonl until compact finishes" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. Resolved — no action needed, already implemented and tested.

## Resolution

**Verified 2026-07-29.** `HistoryRecord` (`src-tauri/src/types.rs`) has no biometric fields on the struct at all; `HistoryService::compact` (`history.rs:276-302`) rewrites every surviving record through that struct, so legacy `embedding`/`centroid_embedding`/`vad_purity`/etc. keys are dropped on the next compaction. Covered by `legacy_history_line_with_embeddings_still_deserializes` (`types.rs:1152-1163`), which asserts the rewritten line contains neither `"embedding"` nor `"centroid"`. No code change needed.

## Findings

- Compact routine: `HistoryService::compact` at `src-tauri/src/services/history.rs:276-302`, run at startup (`src-tauri/src/lib.rs:796`). Loads records via `ensure_loaded` (deserializes each line as `HistoryRecord`), filters tombstones/superseded, then re-serializes and renames the file into place — a full rewrite of every surviving line through the current struct model.
- `HistoryRecord` (`src-tauri/src/types.rs:596-648`) is plain `#[derive(Serialize, Deserialize)]`, no unknown-field passthrough — so legacy keys not on the current struct (`embedding`, `centroid_embedding`, `encrypted_centroid_embedding`, `vad_purity`, `rms_energy`, `quality_score`, etc.) are silently dropped on deserialize and never written back out.
- **Compaction already strips old biometric/voiceprint fields — this is the intended, already-implemented, already-tested behavior** of the 2026-07-16 voiceprint-purge migration (comment at `types.rs:256/280` confirms intent).
- Test coverage exists: `legacy_history_line_with_embeddings_still_deserializes` (`types.rs:1151-1164`) asserts a legacy line with embedding fields deserializes fine and round-trips without those keys; `compact_drops_tombstones_and_superseded` (`history.rs:750`) covers the tombstone/dedup side separately.
- Size estimate: none — no fix required, the described purge is already done and tested.
