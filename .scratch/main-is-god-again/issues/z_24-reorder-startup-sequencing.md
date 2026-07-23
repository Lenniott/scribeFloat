---
title: Reorder startup sequencing so tray isn't gated on background-safe work
labels: [wayfinder:task]
status: closed
assignee: claude
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

## Resolution

`create_tray` now runs immediately after `settings_ctrl`/`rehydrate_hotkeys`/`get_hotkeys` resolve — its only real dependencies. Everything that used to sit between "models_dir created" and `create_tray` (the seed-integrity loop over Whisper/VAD/Sortformer, plus the `bundled_model_available`/`bundled_vad_available` warning checks that depend on seeding having finished) moved into a single `tauri::async_runtime::spawn` block placed right after `create_tray`, matching the existing pattern already used for history compaction. `diarization`/`model`/`model_ctrl` construction stayed synchronous and ahead of the tray — confirmed both constructors are pure in-memory (no disk I/O), so they cost nothing worth backgrounding.

**Not touched:** `prewarm_dictate_window` stays synchronous. It already runs *after* `create_tray` in the original code (so it never gated tray creation to begin with), and it builds a real `WebviewWindow` — Tauri window-builder calls need the main thread, so moving it into `tauri::async_runtime::spawn` (which runs on a tokio worker thread, not necessarily the main thread) would need an `app.run_on_main_thread` hop to do safely. Given it doesn't block the thing this ticket cares about (tray appearance) and I have no running app to verify a main-thread-hop change against, I left it as-is rather than guess at an unverified change to window-creation code. Flagging this explicitly rather than silently declaring it done — worth a follow-up ticket if `setup()`'s total wall-clock time (as opposed to tray-appearance time specifically) becomes a concern.

**Verify:** `cargo build` clean; `cargo test -p ScribeFloat` → 350 passed, 0 failed; `cargo clippy -p ScribeFloat -- -D warnings` clean. Manual relaunch check not performed (no running app in this environment) — reordering was verified by reading the dependency chain directly (settings_ctrl/hotkeys don't touch `models_dir`/`resource_dir` at all) rather than by observation.

**Correction (2026-07-23, found via real `cargo tauri dev` use):** the "not performed" manual check above should have been a blocker, not a caveat. The original `tauri::async_runtime::spawn(async move { ...seed loop... })` ran its synchronous file I/O (existence checks, hashing, potential large copies) directly inside the async block — that occupies a core Tokio runtime worker thread for the whole blocking duration instead of yielding it. Human-observed symptom: opening the main window right after the tray appears showed a 16+ second skeleton, but waiting ~30s first (letting the seed task finish) made it load instantly — a classic "blocking work starving the async runtime's worker pool that window IPC commands also share" signature, not a one-off fluke.

Fixed by wrapping the seed loop in `tokio::task::spawn_blocking` (the pattern already used everywhere else in this codebase for sync work — `commands/scribe.rs`, `commands/settings.rs`, `controllers/dictate.rs`, `controllers/transcribe.rs` — that this ticket should have followed the first time instead of a bare `async_runtime::spawn`). The background task now runs on the dedicated blocking thread pool, leaving async runtime workers free for window IPC regardless of how long seeding takes.

**Lesson:** "moved off the main thread" and "moved off the thing this ticket cares about blocking" are not the same claim as "isolated from everything else that shares the runtime" — should have checked what pool `tauri::async_runtime::spawn` actually uses before calling this done, not just that it wasn't the main thread anymore.

## Comments

- 2026-07-23: Ticketed from [[23-sequential-loading-habits-in-app-startup]]. `blocked_by` 22 only so the purge-removal diff lands first and this ticket doesn't have to re-touch code that's about to be deleted.
- 2026-07-23: Implemented directly against `release/0.3` alongside ticket 22 (not via a worktree agent).
- 2026-07-23: Human reported the app now takes even longer to become usable after this change. Root-caused to blocking I/O inside `async_runtime::spawn` starving the runtime; fixed with `spawn_blocking`. See Correction above.
