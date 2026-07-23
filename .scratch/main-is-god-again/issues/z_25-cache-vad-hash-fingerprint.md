---
title: Stop hashing the VAD model on every launch
labels: [wayfinder:task]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

`dest_needs_bundle_restore` (`services/bundled_models.rs:34-42`) runs a full SHA-256 over the VAD model file on every single startup (`lib.rs:611-613`, marked `hash_at_startup: true` because the file is "tiny — fine to pin every launch"). The file rarely changes. Should this be replaced with a cheap fingerprint (mtime + size) that only falls back to a full hash when the fingerprint changes, or is the VAD file small enough that this genuinely doesn't matter and the ticket should close as "not worth it"?

**Done when:** either (a) a mtime+size fingerprint cache is added so the hash only re-runs when the file's fingerprint changes, or (b) the ticket closes with a measured cost (e.g. actual hash time for the real VAD file) showing it's negligible and not worth the added complexity.

## Resolution

Took option (a). Added `dest_needs_bundle_restore_cached` in `services/bundled_models.rs`: computes the file's (mtime, size) fingerprint and compares it against a sidecar cache file (`.{filename}.integrity`, stored next to the model, format `mtime:size:expected_sha`). If the fingerprint and the `expected_sha` both match the cache, skips the hash entirely and trusts the prior result. Otherwise (missing cache, changed file, or a different `expected_sha` — e.g. a bundled-model version bump shipping a new pinned hash) falls back to the existing full `file_sha256_hex` and writes a fresh cache entry only when the file verifies OK. A stale/tampered cache can only cause a *redundant* re-hash, never suppress a real integrity failure, since any mismatch on mtime/size/sha falls straight through to the full hash path.

Wired into `lib.rs`'s VAD seed-check call site (the only `hash_at_startup: true` entry) in place of the old unconditional `dest_needs_bundle_restore`. Whisper/Sortformer entries are unaffected — they already hash at use-time, not startup, for unrelated reasons (ticket 15).

Designed the cache generically (keyed on dest path + expected sha, not VAD-specific) so [[36-transcribe-cache-whisper-hash]] can reuse the same helper for the Whisper per-batch-item rehash, rather than writing a second cache.

**Verify:** new tests `cached_check_hashes_once_then_trusts_fingerprint`, `cached_check_rehashes_when_file_changes`, `cached_check_rehashes_when_expected_sha_changes`, `cached_check_true_when_dest_missing` in `services/bundled_models.rs`. `cargo test -p ScribeFloat` → 350 passed, 0 failed. `cargo clippy -p ScribeFloat -- -D warnings` clean.

## Comments

- 2026-07-23: Ticketed from [[23-sequential-loading-habits-in-app-startup]]. Lowest-risk of the three startup tickets — pure caching, no behavior change to the integrity guarantee itself.
- 2026-07-23: Implemented directly against `release/0.3` alongside tickets 22/24 (not via a worktree agent).
