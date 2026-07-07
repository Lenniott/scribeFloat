---
id: "0065"
title: Remove stale full-screen capture and scribe views
status: active
---

# Remove stale full-screen capture and scribe views

As a developer maintaining ScribeFloat, I want the old full-screen recording interface removed from the codebase so that dead code doesn't silently run navigation guard logic on every route change and doesn't mislead future contributors into thinking it's an active code path.

## What's stale

The old recording flow used a full-screen capture view that replaced the main content area while recording. This has been superseded by `RecordingStrip` embedded in the note editor, but the old code is still present and partially wired in:

- `src/lib/ui/5_views/capture.svelte` — old full-screen capture wrapper
- `src/lib/ui/5_views/scribe.svelte` — old full-screen recording interface
- `src/lib/ui/5_views/scribe-processing.svelte` — old transcription progress screen
- `src/routes/+layout.svelte` — still imports `CaptureView` and conditionally renders it behind `appState.captureOpen`; still runs two navigation guards that check `captureOpen` and `captureLeaveGuard` on every route change
- `src/lib/stores/appState.svelte.ts` — still carries `captureOpen`, `captureLeaveGuard`, and `captureVisitKey` fields

`captureOpen` is never set to `true` in the current flow, so the views are unreachable — but the guard logic still executes on every navigation.

## What to do

Remove the three view files. Remove the `CaptureView` import and conditional render block from `+layout.svelte`. Remove the two `captureOpen`/`captureLeaveGuard` guard branches from the navigation logic. Remove the three stale fields from `appState`.

## Notes

- Verify `captureOpen` is genuinely never set before deleting — search the full `src/` tree for any remaining setter
- The Tauri backend commands (`scribe_start`, `scribe_stop_and_save`, etc.) are still used by `RecordingStrip` — do not touch those
- `scribe-processing.svelte` may contain progress bar logic worth salvaging into the `RecordingStrip` transcribing phase before deleting (see story 0064)
