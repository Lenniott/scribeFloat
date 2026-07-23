---
title: Always delete legacy voice Keychain key
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Does startup always remove the old voiceprint encryption key from Keychain — even when the `voiceprints/` folder is already gone — so the abandoned feature leaves no Keychain ghost?

**Done when:** Key delete is not gated only on “profiles dir removed”; safe if key already absent; matches “as if voiceprint never happened.”

## Spec (to-spec)

Evidence: security S4 (`research/security-review.md`) — startup only calls `platform::delete_voice_crypto_key()` when `report.profiles_dir_removed` is true (`src-tauri/src/lib.rs`). If `voiceprints/` was already gone (manual delete, prior partial purge) but the Keychain entry remains, startup never deletes it. Clips-only leftover also does not trigger delete. `delete_voice_crypto_key` itself already treats “key not found” as success (`src-tauri/src/platform/mod.rs`).

**In this ticket:** always attempt Keychain key delete on startup, independent of whether the profiles (or clips) directory was removed this run.  
**Not this ticket:** history embedding compaction / on-disk biometric fields → Known issues / separate hygiene. Sortformer SHA → (15). Per-window IPC → (16). ADR wording → (18).

### Blast radius (human — binding)

Voiceprint **never shipped**. It only ever existed in this exploration / branch chaos on the human’s machine. There is no released-user upgrade path and no fleet of installs with orphan keys. This ticket is **local hygiene** so the spine / `main` leave no Keychain ghost — not a product bug affecting other people. Agents must not re-explain or re-escalate this as “every Mac that ever ran the app.” After this ticket closes, drop voiceprint from merge narration (see map Decisions).

### What “as if voiceprint never happened” means here

Voiceprint is abandoned (exploration-only). Names may already have been imported into the plain speaker store. Profile/clip dirs may or may not still exist on the human’s machine. The AES key in Keychain must not linger as a ghost — even when there is nothing left on disk to encrypt.

### Aggression (agreed)

**(1) Always delete every startup** — After the filesystem purge runs (or no-ops), always call `delete_voice_crypto_key()`. Drop the `if report.profiles_dir_removed` gate. Keep today’s error behaviour: missing key = success; other Keychain errors = `warn` and continue (do not block app start). Update the comment in `legacy_voice_purge.rs` that still says the caller deletes the key only when `profiles_dir_removed`.

### Code cut (fixed once aggression agreed)

1. In `lib.rs` startup purge block: call `delete_voice_crypto_key()` unconditionally after `purge_legacy_voice_data`.
2. Keep warn-and-continue on non-missing Keychain errors.
3. Fix stale comment in `legacy_voice_purge.rs` (filesystem-only module; caller always deletes key).
4. No change to `delete_voice_crypto_key` behaviour (missing = Ok; Windows stub stays no-op — key was never stored there).
5. No new Keychain integration test required (platform call; unit tests stay filesystem-only).

### Done when

1. Startup always invokes key delete; not gated on `profiles_dir_removed` (or clips).
2. Missing key remains success; other failures still warn, do not crash startup.
3. Comment in `legacy_voice_purge.rs` matches the new caller contract.
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass.
5. Approach recorded in Resolution.

## Resolution

Aggression **(1)** on `feature/0.3/embeds`.

| Cut | Result |
|---|---|
| Gate on `profiles_dir_removed` | Removed — `delete_voice_crypto_key()` always called after filesystem purge |
| Missing key | Still success (unchanged helper) |
| Other Keychain errors | Still warn-and-continue |
| `legacy_voice_purge.rs` docs | Caller always deletes key; module stays filesystem-only |
| Blast radius | Local hygiene only (voiceprint never shipped) |

**Verify:** `cargo test -p ScribeFloat` → 336 passed, 5 ignored; `cargo clippy -p ScribeFloat -- -D warnings` clean.

## Comments

- 2026-07-19: claimed; to-spec drafted; waiting on human OK for aggression **(1)**.
- 2026-07-19: human clarified voiceprint never released — exploration-only on their machine; recorded under Blast radius + map Decisions.
- 2026-07-19: human OK’d **(1)** (“go”); implemented and closed.
