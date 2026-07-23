---
title: Eagerly preload Whisper at startup instead of waiting for first use
labels: [wayfinder:task]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

Ticket 30 moved Dictate's Whisper preload to key-down time, which helps every *subsequent* Dictate session in a run (the context is cached for the app's lifetime in `ModelService::loaded_contexts`), but the very first capture after a fresh app launch still pays the full cold-load cost since nothing warms the model until the user actually acts. Human, after the ticket-24 fix landed: "The initial model load on Dictate was very slow. I just don't see the point of not just keeping it in RAM when it's idle. It doesn't actually take up that much space, right?"

**Done when:** Whisper Small is preloaded into `loaded_contexts` during startup idle time, backgrounded so it never blocks the tray or window IPC (must use `spawn_blocking`, not a bare `async_runtime::spawn` — see ticket 24's correction for why that distinction matters).

## Resolution

Confirmed the premise: Whisper Small is ~181 MB on disk, and `ModelService::preload_context` already caches the loaded `WhisperContext` for the app's entire lifetime (`loaded_contexts: Mutex<HashMap<PathBuf, Arc<WhisperContext>>>`) — its own doc comment already says "Blocking; call from a background thread," meaning this was designed to be called eagerly, just never wired up to be.

Added one line — `model_seed_bg.preload_context(&default_path)` — at the end of the existing model-seed `spawn_blocking` closure in `lib.rs` (right after the seed loop confirms the model file is present/healed, and after the availability warnings). Runs on the blocking thread pool, same as the seed loop itself, so it can't starve window IPC the way the pre-fix seed loop did.

Not preloaded: VAD (tiny, loaded per-transcription-call already, not worth a separate warm-up) and Sortformer (large — ~469 MB — and only used by Record/Upload's diarization, not Dictate's hot path; preloading it unconditionally would cost real startup CPU/RAM for users who mostly just Dictate and never Record/Upload. Left as a candidate for a future ticket if Record's cold-diarization-load ever becomes a similar complaint).

**Verify:** `cargo build` clean; `cargo test -p ScribeFloat` → 350 passed, 0 failed; `cargo clippy -p ScribeFloat -- -D warnings` clean. Manual timing not measured in this environment (no real Whisper model file here) — the human should notice the first Dictate press of a session no longer paying the cold-load wait, matching the already-warm feel of subsequent presses.

## Comments

- 2026-07-23: Raised directly by the human after the ticket-24 fix landed and they tested a real `cargo tauri dev` session.
