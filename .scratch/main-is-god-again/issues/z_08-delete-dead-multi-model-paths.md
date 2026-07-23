---
title: Delete dead multi-model paths
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by:
  - "05-architecture-single-model-review.md"
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Using the single-model inventory from *Architecture and single-model review* and the sort in *Sort findings into merge-blockers vs Known issues*, what is the exact deletion set for dead multi-model UI and code, and has it been removed so no user-facing or reachable path still offers multi-model download/choose?

## Spec (to-spec)

Inventory source: [architecture-single-model-review.md](../research/architecture-single-model-review.md) §B.  
**In this ticket:** code + error strings only.  
**Not this ticket:** README / PRIVACY / site / CONTEXT → *Bundle-only models* (12). ADR-0010 / aspirational marks → *Mark and amend ADRs* (18). Stale skills/plans → Known issues. VAD runtime fetch → ticket 12.

### Aggression (agreed)

**Full collapse** — delete chooser-shaped API; do not keep a 1-entry catalog.

### Done when

1. No `selected_model_id` / `scribe_model_path` / `select_model` / Upload `model_id` chooser path remains reachable.
2. Record, Dictate, and Upload all resolve the **same** bundled Small Whisper file.
3. User-visible errors never mention Settings → Models or “re-download from Settings.”
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass.
5. Deleted UI from inventory **A** still absent (smoke grep).

## Resolution

Full collapse implemented on `feature/0.3/embeds` working tree.

| Cut | Result |
|---|---|
| `Config.selected_model_id` / `scribe_model_path` | Removed; legacy keys ignored on read (serde) |
| `ModelController::select_model` | Deleted; controller is VAD status only |
| Startup auto-select in `lib.rs` | Replaced with “warn if bundled missing” |
| `MODEL_CATALOG` / id APIs | Collapsed to `SMALL_MODEL_FILENAME` + `SMALL_MODEL_SHA256` + `default_model_path()` / `bundled_model_available()` |
| Scribe / Dictate / Upload resolvers | All use `default_model_path()` |
| Upload `model_id` IPC + FE `modelId` | Removed |
| Error / integrity copy | Points at reinstall / bundled seed failure |
| `ModelDownloadEvent` comment | VAD-only; not a Whisper chooser channel |

**Verify:** `cargo test -p ScribeFloat --lib` → 333 passed; `cargo clippy -p ScribeFloat -- -D warnings` clean. Inventory A UI still absent under `src/`.

## Comments

- 2026-07-19: claimed; to-spec drafted; human chose full collapse (1); implemented and closed.
