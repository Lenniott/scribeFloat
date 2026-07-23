---
title: Reorder startup sequencing so tray isn't gated on background-safe work
labels: [wayfinder:task]
status: open
assignee:
blocked_by:
  - "22-remove-legacy-voice-purge-from-startup.md"
parent: MAP.md
---

## Question

In `src-tauri/src/lib.rs::run()`'s `.setup()` closure, `create_tray` (`lib.rs:674`) only runs after config load, the model-seed integrity loop (`lib.rs:591-641`), and diarization/model/settings construction all complete. Once ticket 22 removes the legacy purge, and `prewarm_dictate_window` (`lib.rs:756`) still runs synchronously before `setup()` returns — how should these be reordered/backgrounded so the tray (the app's primary "is it running" signal, since there's no default window) appears as early as possible?

**Done when:**
1. `create_tray` runs as early as its real dependencies (hotkeys, config) allow — not after model-seed checks.
2. Model-seed integrity checks (`lib.rs:591-641`) and `prewarm_dictate_window` (`lib.rs:756`) run via `tauri::async_runtime::spawn` (matching the existing pattern at `lib.rs:766-801`), or are otherwise moved off the path that gates tray creation.
3. Correctness preserved: onboarding-window logic (`is_first_run`, `lib.rs:753-755`) still has whatever it needs by the time it runs; anything the tray/menu depends on (open_hotkey, settings_ctrl) still resolves before `create_tray`.
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass; a manual relaunch check confirms the tray appears quickly (see Silicon smoke pattern from ticket 09).
5. Approach recorded in Resolution.

## Comments

- 2026-07-23: Ticketed from [[23-sequential-loading-habits-in-app-startup]]. `blocked_by` 22 only so the purge-removal diff lands first and this ticket doesn't have to re-touch code that's about to be deleted.
