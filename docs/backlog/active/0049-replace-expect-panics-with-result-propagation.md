---
id: "0049"
title: Replace invariant .expect() calls with Result propagation
status: active
priority: low
---

# Replace invariant `.expect()` calls with `Result` propagation

## Problem

Two production code paths use `.expect()` on state that *should* always be present but is not guaranteed by the type system:

**Location 1 — `src-tauri/src/controllers/scribe.rs` ~line 425:**
```rust
let session = self.session.lock().unwrap();
let s = session.as_ref().expect("session exists when Recording");
```

**Location 2 — `src-tauri/src/controllers/dictate.rs` ~line 736:**
```rust
let session = self.session.lock().unwrap();
let s = session.as_ref().expect("session exists when Recording");
```

Both sites assume the state machine is in `Recording` state, so `session` must be `Some`. This invariant is upheld by the state machine transitions in normal operation, but `.expect()` will **panic the whole process** if ever violated — for example, if a concurrent cancel races the lock acquisition window. A panic in a Tokio task or `spawn_blocking` closure crashes the transcription silently from the user's perspective.

The correct pattern is to propagate `None` as an `AppError::Internal` and emit an error event, matching how other unexpected-state cases are handled in both controllers.

## Fix

Replace each `.expect()` with `ok_or_else` + `?`:

```rust
// Before:
let s = session.as_ref().expect("session exists when Recording");

// After:
let s = session.as_ref().ok_or_else(|| {
    AppError::Internal("session missing in Recording state".to_string())
})?;
```

The surrounding function must already return `Result<_, AppError>` (confirm this — if not, adjust the return type). The `?` propagates the error up to the caller which emits an error event to the frontend.

## Scope

- Two files: `src-tauri/src/controllers/scribe.rs` and `src-tauri/src/controllers/dictate.rs`.
- Search both files for all `.expect(` calls. Fix any that guard runtime state (e.g., `Option` field accessed under a lock). Leave `.expect()` calls that guard **compile-time invariants** (static regex, hardcoded default values) — those are acceptable.
- Do not change any public API or event shapes.

## Acceptance criteria

- [ ] Neither `scribe.rs` nor `dictate.rs` contains `.expect()` on a state `Option` field accessed under a lock.
- [ ] All such sites propagate `AppError::Internal` via `?`.
- [ ] If a `None` path is hit at runtime, the controller emits an error event (not a panic).
- [ ] `cargo test -p scribefloat` passes.
- [ ] `cargo clippy -- -D warnings` passes.

## Notes

- `Mutex::lock().unwrap()` (for poison recovery) is a separate concern — do not change those. Focus only on `Option::expect` / `Option::unwrap` on state fields.
- After this change, the only remaining `.expect()` calls in the controllers should be on hardcoded static data (model names, regex literals) — not on runtime-derived `Option` values.
- This is a defensive hardening story, not a fix for a known bug. The existing invariant is likely correct; this removes the risk of an unexpected panic if the invariant is ever violated by a future change.
