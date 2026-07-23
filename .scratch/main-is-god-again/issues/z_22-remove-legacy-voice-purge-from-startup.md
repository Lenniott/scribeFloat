---
title: Remove legacy voice purge from startup
labels: [wayfinder:task]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

`purge_legacy_voice_data` (`src-tauri/src/services/legacy_voice_purge.rs`) plus its call site in `lib.rs` setup (`lib.rs:706-727`) run a filesystem scan (`voiceprints/`, `voiceprint_clips/`) on *every* launch, forever, even though this is a one-time migration for a feature that never shipped ([[14-always-delete-legacy-voice-keychain-key]] already made the Keychain-key delete unconditional). Human confirmed (2026-07-23, load-performance review session): this isn't a migration to keep running — delete it outright rather than reorder/background it.

**Done when:**
1. `purge_legacy_voice_data` call site and the module itself are deleted (not just backgrounded) — including its `#[cfg(test)]` tests.
2. The Keychain-key delete from ticket 14 stays (it's cheap and still relevant — no released voiceprint users, but keeping it costs nothing and ticket 14 already reasoned through why it stays unconditional).
3. `speaker_names.json` load (`lib.rs:703-705`) is untouched — that's the plain speaker-name store, unrelated to the purge.
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass with the module gone.
5. Approach recorded in Resolution.

## Resolution

Deleted `src-tauri/src/services/legacy_voice_purge.rs` entirely and its `pub mod legacy_voice_purge;` line in `services/mod.rs`. In `lib.rs` setup, replaced the whole purge block (report handling + conditional Keychain delete) with just the unconditional Keychain-key delete call, matching ticket 14's existing reasoning — a two-line block now, no filesystem scan on every launch. `speaker_names.json` load untouched, sits right above where the purge block used to be.

**Verify:** `cargo test -p ScribeFloat` → 350 passed, 0 failed (5 fewer than before — the deleted module's own tests; no other regressions). `cargo clippy -p ScribeFloat -- -D warnings` clean.

## Comments

- 2026-07-23: Ticketed out of a load-performance review session ([[23-sequential-loading-habits-in-app-startup]]). Human: "this isn't a thing that needs to happen now... we can get rid of this."
- 2026-07-23: Implemented directly against `release/0.3` (not via a worktree agent, after the Dictate cluster showed hand-reconciliation was needed anyway for stale worktrees).
