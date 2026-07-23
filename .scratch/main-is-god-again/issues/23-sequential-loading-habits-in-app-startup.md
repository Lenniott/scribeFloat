---
title: Sequential-loading habits in app startup
labels: [wayfinder:research]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

App startup was flagged as slow. Setting aside model *load time* itself (Whisper/Sortformer/VAD stay lazy-loaded on first use, confirmed fine) — where does startup do things sequentially on the main thread that could be parallel, backgrounded, or cached, and how much of that is gating the tray icon (the app's only always-present UI)?

## Resolution

Traced in full — see [research/startup-sequential-loading.md](../research/startup-sequential-loading.md). Confirmed: model loads are already lazy (not the problem); the problem is that independent, file-I/O-bound setup steps (config load, model-seed integrity checks incl. an unconditional VAD SHA-256 every launch, legacy voice purge, dictate-window prewarm) are chained sequentially inside one `.setup()` closure and all gate tray creation, even though the codebase already has and correctly uses `tauri::async_runtime::spawn` for the *last* group of startup work (history compaction/scan). The habit is inconsistent application of a pattern that already exists in the same file.

Spun off:
- [[22-remove-legacy-voice-purge-from-startup]] — delete outright, not reorder
- [[24-reorder-startup-sequencing]] — stop gating tray on model-seed checks / prewarm
- [[25-cache-vad-hash-fingerprint]] — stop hashing VAD file every launch

## Comments

- 2026-07-23: Resolved directly from the load-performance review trace already done this session (Explore agent reads of `lib.rs`, `services/bundled_models.rs`, `services/model.rs`, `services/diarization.rs`, `services/history.rs`) — no separate subagent re-dispatch needed since the trace was already verified against current line numbers.
