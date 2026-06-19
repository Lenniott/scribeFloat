---
id: "0050"
title: Remove stale TODO comment and fix prewarm log level
status: active
priority: low
---

# Housekeeping: stale TODO and wrong prewarm log level

Two small code-quality issues found during the backend audit. No behaviour change required for either.

## Issue 1 — Stale `TODO(S1)` comment in `model.rs`

`src-tauri/src/services/model.rs` contains a comment (around the SHA-256 verification block) that reads approximately:

```rust
// TODO(S1) to enable pinning
```

This comment is stale. All five model catalog entries in `ModelCatalogItem` already have `sha256` values populated (pinning is already enabled). The comment refers to work that was completed and falsely implies SHA-256 verification is disabled or optional.

**Fix:** Delete the stale TODO comment. Do not change any code around it.

## Issue 2 — Prewarm failure logged at `debug` instead of `warn` in `lib.rs`

`src-tauri/src/lib.rs` around line 265 logs a Dictate window prewarm failure at `debug` level:

```rust
if let Err(e) = prewarm_dictate_window(&app) {
    tracing::debug!(error = %e, "dictate prewarm failed");
}
```

Prewarm failure means the Dictate window may not open instantly on first use — a degraded UX. This should be logged at `warn` so it is visible in release logs when diagnosing responsiveness issues.

**Fix:**
```rust
if let Err(e) = prewarm_dictate_window(&app) {
    tracing::warn!(error = %e, "dictate prewarm failed");
}
```

## Acceptance criteria

- [ ] The stale `TODO(S1)` comment is removed from `model.rs`.
- [ ] The prewarm failure is logged at `tracing::warn!` in `lib.rs`.
- [ ] No other code changes in either file.
- [ ] `cargo clippy -- -D warnings` passes.
- [ ] `cargo test -p scribefloat` passes.

## Notes

- Both changes are one-liners. Keep them in the same commit to avoid noise.
- When removing the TODO comment, check whether the surrounding comment block still makes sense without it — trim any orphaned context that only existed to explain the now-removed TODO.
