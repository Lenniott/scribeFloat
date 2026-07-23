---
title: Stop re-hashing the Whisper model on every batch item in Transcribe
labels: [wayfinder:task]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

## Question

`transcribe_pcm_with_progress` (`services/model.rs:436-444`) runs a synchronous SHA-256 hash of the Whisper model file immediately before every single inference call — meaning a 20-item batch upload hashes the same ~181MB file 20 times. The hash result is input-independent (same model file, doesn't change mid-batch); this is the same class of fix as the VAD-hash caching ticket for startup ([[25-cache-vad-hash-fingerprint]]).

**Done when:** the integrity check runs once per run (or once per model-file fingerprint change, matching ticket 25's approach) rather than once per batch item; batch transcription of N items no longer re-hashes the model N times. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass; approach recorded in Resolution.

## Comments

- 2026-07-23: Spun off [[29-transcribe-flow-sequential-loading]]. Likely shares an implementation with [[25-cache-vad-hash-fingerprint]] — worth a combined "model integrity fingerprint cache" ticket if that's cleaner than two separate caches; leaving as two tickets since they hit different call paths (startup seed loop vs. per-item transcription) until someone reads both closely enough to confirm they can share one cache.
