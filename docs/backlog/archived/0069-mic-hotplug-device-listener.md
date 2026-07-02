---
id: "0069"
title: Refresh mic list when input devices are plugged or unplugged
status: done
---

# Refresh mic list when input devices are plugged or unplugged

USB/external mics do not appear in the Scribe mic dropdown until the user refocuses the window or reopens settings (`refreshMicOptions()` on focus). Mid-recording unplug is handled separately (cpal stream error → reconnect to default); this story is about **discovery** of newly attached devices without manual refresh.

## What to build

- Platform listener for input-device topology changes (CoreAudio on macOS; equivalent or no-op stub on Windows)
- Emit `audio://input-devices-changed` (or reuse an existing event name if one exists)
- Frontend: `scribeController.refreshMicOptions()` on that event (settings popover / mic dropdown stay current)

## Notes

- Do not duplicate mid-recording fallback logic — that lives in `MicSession::reconnect_to_default_input` and `scribe://mic-fallback`
- Update `docs/engineering/platform-rules.md` and `docs/action-flows.md` when shipping
