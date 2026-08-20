# Record Edge Glow Overlay

Parked from [main-is-god-known-issues](../../.scratch/main-is-god-known-issues/MAP.md)
ticket 09 (related surface of the Dictate overlay experiment). **Not scheduled
work.** Do not implement without a wayfinder ticket. There is no glow window,
`?view=recording-glow` route, or capability set in the code today.

## Summary
- Difficulty: **medium-high**, mostly because macOS full-screen Spaces are fussy.
- Build a dedicated, transparent, click-through overlay window shown only while **Record** is actively recording.
- It should not take focus, should not block clicks, and should hide as soon as Record leaves `Recording`.
- Target behavior: glow appears on the **active monitor / active Space** where the user is working, including common macOS full-screen app usage.

## Key Changes
- Add a new satellite route/view, e.g. `?view=recording-glow`, rendering only a full-window edge glow.
- Create/show the glow from Rust when `ScribeController` emits `Recording`; hide/destroy it on `Idle`, `Transcribing`, `Done`, `Error`, cancel, and shutdown cleanup.
- Build the overlay window with:
  - `transparent(true)`
  - `decorations(false)`
  - `always_on_top(true)`
  - `skip_taskbar(true)`
  - `focusable(false)`
  - `set_ignore_cursor_events(true)`
  - `visible_on_all_workspaces(true)` for macOS full-screen/Spaces behavior
- Size and position it to the active monitor. First implementation should use cursor/current monitor at Record start as the “screen being used”; if that proves wrong in smoke, add a macOS frontmost-window screen lookup.
- Exclude the glow label from Dock/taskbar visibility logic the same way Dictate is excluded.
- Add a minimal capability for the glow window: `core:default` plus no app commands unless the view must call `settings_get_theme_mode`.
- Visual treatment: subtle destructive-token edge cue, not a panel:
  - transparent body
  - 2-4px edge line plus soft inset glow
  - opacity-only pulse while recording
  - `pointer-events: none`
  - respect `prefers-reduced-motion`

## Feasibility Notes
- Tauri 2.11 already exposes the required non-interaction primitives: `focusable(false)` and `set_ignore_cursor_events(true)`.
- Full-screen Spaces support is the riskiest part. `visible_on_all_workspaces(true)` is the first pass; if macOS still hides it behind full-screen apps, add a small macOS-specific window-level/collection-behavior helper.
- Transparent macOS behavior should be verified early. If a full transparent webview does not render correctly, enable the required Tauri macOS private API/config path explicitly.

## Test Plan
- Rust tests:
  - glow show is triggered only for `ScribeState::Recording`
  - glow hide is triggered for all non-recording terminal/intermediate states
  - Dock visibility ignores both Dictate and glow overlay labels
- Frontend test:
  - glow view renders no interactive controls and applies reduced-motion behavior
- Manual smoke on macOS:
  - Start Record while another app is focused
  - Confirm focus stays in that app
  - Click/drag/type through the glow
  - Stop/cancel Record and confirm glow disappears
  - Repeat in a full-screen app Space
  - Repeat on an external monitor
- Run:
  - `cargo test -p ScribeFloat acl_capabilities`
  - relevant Svelte/Vitest view tests
  - manual `cargo tauri dev` smoke for real window behavior

## Assumptions
- This is for **Record** mode only, not Dictate.
- No Settings toggle in v1.
- Glow is intentionally visual-only: no buttons, no text, no interaction.
- If active-monitor detection is ambiguous, cursor monitor at Record start is the default.
