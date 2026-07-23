---
title: Stop re-hashing the Whisper model on every batch item in Transcribe
labels: [wayfinder:task]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

`transcribe_pcm_with_progress` (`services/model.rs:436-444`) runs a synchronous SHA-256 hash of the Whisper model file immediately before every single inference call — meaning a 20-item batch upload hashes the same ~181MB file 20 times. The hash result is input-independent (same model file, doesn't change mid-batch); this is the same class of fix as the VAD-hash caching ticket for startup ([[25-cache-vad-hash-fingerprint]]).

**Done when:** the integrity check runs once per run (or once per model-file fingerprint change, matching ticket 25's approach) rather than once per batch item; batch transcription of N items no longer re-hashes the model N times. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass; approach recorded in Resolution.

## Resolution

**Already solved on `release/0.3` — no change needed.** The research this ticket was spun from traced the stale worktree base (278 commits behind), which genuinely re-hashed on every call. Current `release/0.3` already has `ModelService::model_integrity_ok_cached` (`services/model.rs:216-237`), backed by an in-memory `verified_models: Mutex<HashMap<PathBuf, (SystemTime, u64)>>` cache (mtime+size fingerprint, same design as [[25-cache-vad-hash-fingerprint]]'s disk-persisted cache) — `transcribe_pcm_with_progress` (`model.rs:436-444`) already calls it before falling back to a full hash, and `ensure_whisper_integrity` explicitly invalidates the cache entry when a heal/restore happens. This must have landed in one of the commits between the ticket's stale research base and now, independent of this effort.

Confirmed no gap remains: `model_integrity_ok_cached` is called first every time, only falling through to a real `hash_matches` (full SHA-256) when the cache misses or the file's fingerprint changed. A 20-item batch with an unchanged model file hits the cache 19 times, hashes once.

Not merged with ticket 25's disk-persisted cache — they're genuinely different (this one is in-memory/per-process via `ModelService`'s own state; ticket 25's is disk-persisted via a sidecar file next to the model, needed because `bundled_models::dest_needs_bundle_restore_cached` runs from a bare `lib.rs` setup closure with no long-lived service instance to hold an in-memory cache across app restarts). Both independently solve the same underlying "don't rehash an unchanged file" problem for their respective call sites.

**Verify:** no code changed; `cargo test -p ScribeFloat` — 350 passed, 0 failed (same run as tickets 22/24/25/35). `cargo clippy -p ScribeFloat -- -D warnings` clean.

## Comments

- 2026-07-23: Spun off [[29-transcribe-flow-sequential-loading]]. Likely shares an implementation with [[25-cache-vad-hash-fingerprint]] — worth a combined "model integrity fingerprint cache" ticket if that's cleaner than two separate caches; leaving as two tickets since they hit different call paths (startup seed loop vs. per-item transcription) until someone reads both closely enough to confirm they can share one cache.
- 2026-07-23: Investigated directly against `release/0.3` (not via the worktree agent, which never got this far before hitting the stale-base blocker) — found the fix already present, unrelated to this effort.
