# Research: sequential-loading habits in app startup

**Date:** 2026-07-23. Traced directly in a load-performance review session (Explore agent, read-only), not a background research subagent — findings verified against the current tree.

## What happens

`src-tauri/src/lib.rs::run()` → `.setup()` closure (`lib.rs:559-804`) runs everything below **sequentially, on the main thread**, before the tray icon (the app's only always-present UI, since `tauri.conf.json` declares `"windows": []`) appears:

1. `ConfigService::load` — sync file read + JSON parse (`lib.rs:569`, `services/config.rs:13-37`). Small file, negligible.
2. Windows save-folder migration check (fs) — `lib.rs:572-584`.
3. Model-seed integrity loop (`lib.rs:591-641`): existence/empty checks for Whisper (~181MB) and Sortformer (~469MB); a **full SHA-256 hash of the VAD model on every single launch** (`services/bundled_models.rs:34-42`, comment at `lib.rs:613` says "tiny — fine to pin every launch" — true per-file, but it's still unconditional repeated work for a file that essentially never changes). Whisper/Sortformer only get missing/empty checks here (hashing those synchronously was previously "~30-40s of 100% CPU before tray appeared" — already fixed once, see ticket 15).
4. `DiarizationService`/`ModelService` construction — cheap, no model load (paths only).
5. `SettingsController::new` + `rehydrate_hotkeys()`.
6. `create_tray(app, &open_hotkey)` — **only reachable after everything above**.
7. Three controller constructions (Scribe/Dictate/Transcribe) — cheap.
8. `SpeakerNameService::load` — sync JSON read, small.
9. `purge_legacy_voice_data` — sync `read_dir`/`remove_dir_all` scan (`lib.rs:710-727`) — see [[22-remove-legacy-voice-purge-from-startup]], being deleted outright rather than fixed.
10. `dictate_ctrl.ensure_key_listener()` — spawns background threads, non-blocking. Good pattern.
11. `prewarm_dictate_window` — **synchronously builds a whole hidden WebviewWindow** (full renderer construction) before `setup()` returns (`lib.rs:287-309, 756`).
12. Only at the very end: history compaction + incomplete-session scan + dictate-temp-wav salvage are `tauri::async_runtime::spawn`ed onto a background task (`lib.rs:766-801`), with an explicit comment explaining why ("large histories can take 100-500ms... never block the Tauri event loop"). **This is the pattern the rest of the list should follow and doesn't.**

## Model loading itself is NOT the problem

Whisper/Sortformer/VAD are not loaded into memory at startup — `ModelService`/`DiarizationService` constructors just store paths (`services/model.rs:86-104`, `services/diarization.rs:138-148`). Actual model load is lazy, triggered only by `preload_context` calls from user action (`controllers/scribe.rs:426`, `controllers/dictate.rs:608`) or `start_live_session`. This matches the human's steer: don't look at model load times, look at the *sequencing habit* around them.

## The habit, named

Independent, file-I/O-bound setup steps are written as one long sequential chain inside a single `.setup()` closure, gating the tray (the app's "is it running" signal) on all of them — even though the codebase already has, and uses correctly at the very bottom, the pattern for doing the opposite (`tauri::async_runtime::spawn`). The good pattern exists; it just wasn't applied uniformly as new steps were added over time.

## Decisions made from this trace

- Delete `purge_legacy_voice_data` entirely rather than reorder it → [[22-remove-legacy-voice-purge-from-startup]]
- Reorder so tray creation isn't gated on model-seed checks / dictate-window prewarm → [[24-reorder-startup-sequencing]]
- Stop hashing the VAD file on every launch; fingerprint it instead → [[25-cache-vad-hash-fingerprint]]
