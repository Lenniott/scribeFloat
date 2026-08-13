# Dictate overlay on macOS full-screen / Spaces

Parked from [main-is-god-known-issues](../../.scratch/main-is-god-known-issues/MAP.md) ticket 09. Experiment / fiddly platform work — expect iteration on a dedicated branch, not a one-shot triage fix.

## Summary
- Difficulty: **medium**, mostly because macOS full-screen Spaces behavior is fussy.
- Symptom: Dictate HUD disappears when a full-screen app is frontmost on another Space; **capture still works** (hotkey + audio).
- Root cause direction: window never gets `NSWindowCollectionBehaviorCanJoinAllSpaces` / `FullScreenAuxiliary`. `always_on_top` only layers within a Space.

## Why it's parked here
Needs real on-device testing across Spaces / Split View / macOS versions. Related surface already sketched in [`recordingGlow.md`](recordingGlow.md) (`visible_on_all_workspaces`, collection-behavior helpers). Better as an experiment branch than a Now ticket in a triage effort.

## Research already done
Grounded findings live in the closed triage ticket:
`.scratch/main-is-god-known-issues/issues/z_09-dictate-overlay-fullscreen-flaky.md`

Key anchors:
- Window create/show: `prewarm_dictate_window` / `open_dictate_window` in `src-tauri/src/lib.rs`
- Existing raw-objc pattern to copy: `platform/window_impl.rs` (Dock icon only today — no window collection behavior yet)

## Suggested future destination
- Add a macOS helper (e.g. `set_dictate_collection_behavior`) after build in both prewarm and open paths.
- Combine collection behavior with an appropriate window level; verify manually on real full-screen Spaces.
- Consider sharing the helper with any Record edge-glow overlay work.
