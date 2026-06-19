---
id: "0048"
title: Add unit tests for DictateKeyTracker state machine
status: active
priority: medium
---

# Add unit tests for `DictateKeyTracker` state machine

## Problem

`DictateKeyTracker` in `src-tauri/src/controllers/dictate.rs` implements a 6-state machine that controls push-to-talk and toggle-record gestures. It has four timing constants and several guard conditions. There are **zero unit tests** for it. A regression in any transition is invisible until a human tests the hardware interaction.

### States

```
Idle
FirstPressed
AwaitingSecondTap
SecondHeldArming
HoldRecordingAwaitingRelease
ToggleRecording
```

### Timing constants (approximate — confirm exact values in source)

| Constant | Value |
|---|---|
| `FIRST_PRESS_MAX_MS` | 300 ms |
| `HOLD_THRESHOLD_MS` | 500 ms |
| `DOUBLE_TAP_WINDOW_MS` | 400 ms |
| `TOGGLE_STOP_COOLDOWN_MS` | 1000 ms |

### Inputs the tracker accepts

- `key_down(now_ms: u64)` — modifier key pressed
- `key_up(now_ms: u64)` — modifier key released

### Outputs / side-effects

Each transition may produce an `Action` (or equivalent): `StartRecording`, `StopRecording`, `None`. Confirm the exact enum/output type in the source before writing tests.

## What to test

Cover every named transition path. Suggested test cases:

| Scenario | Inputs | Expected output |
|---|---|---|
| Short tap — no action | down(0), up(100) | None |
| Hold — starts recording | down(0), up(600) | StartRecording at ~500 ms |
| Hold then release — stops recording | down(0), up(600) [triggers start], up(601) or final up | StopRecording |
| Double-tap within window — toggle on | down(0), up(100), down(200), up(300) | StartRecording (toggle) |
| Double-tap — toggle off after cooldown | ...toggle on, then second double-tap after cooldown | StopRecording |
| Double-tap after window expires — treated as two short taps | down(0), up(100), down(600), up(700) | None (second tap is a new first-press) |
| Toggle stop blocked by cooldown | toggle on, immediate second double-tap before cooldown | StopRecording suppressed |

## How to write the tests

`DictateKeyTracker` should be testable with **no Tauri runtime** — it must not call any `AppHandle` methods directly inside the state machine logic. If it currently does, refactor the tracker to return `Action` values that the controller's `handle_key_event` method dispatches. This keeps the tracker pure and testable.

```rust
// Target API for testing:
let mut tracker = DictateKeyTracker::new();
let action = tracker.key_down(0);
assert_eq!(action, TrackerAction::None);
let action = tracker.key_up(600);
assert_eq!(action, TrackerAction::StartRecording);
```

Tests live in `src-tauri/src/controllers/dictate.rs` `#[cfg(test)]` block.

## Acceptance criteria

- [ ] At least 8 test cases covering the scenarios in the table above.
- [ ] Tests run with `cargo test -p scribefloat` — no Tauri runtime, no audio devices, no file I/O.
- [ ] If refactoring is required to make the tracker testable (action-return pattern), the controller's behaviour at all existing call sites is unchanged.
- [ ] The timing constants are referenced by name in tests (not hardcoded magic numbers) so that changing a constant updates test semantics automatically.
- [ ] `cargo clippy -- -D warnings` passes.

## Notes

- Do not mock time with an external crate. Pass `now_ms: u64` as a parameter (the tracker already does this — confirm in source). Deterministic time is all that is needed.
- Do not test audio device interaction or key event capture — those are integration concerns. Only test the state machine logic.
- If the tracker currently mutates app state directly rather than returning actions, the refactor required by this story is small: replace direct `ctrl.start()` / `ctrl.stop()` calls inside `DictateKeyTracker` with `return TrackerAction::Start` and let the caller dispatch. The tracker should be a pure state machine.
