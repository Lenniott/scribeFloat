---
title: Remove legacy voice purge from startup
labels: [wayfinder:task]
status: open
assignee:
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

## Comments

- 2026-07-23: Ticketed out of a load-performance review session ([[23-sequential-loading-habits-in-app-startup]]). Human: "this isn't a thing that needs to happen now... we can get rid of this."
