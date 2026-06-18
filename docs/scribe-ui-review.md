# Scribe UI Review

> Read this before touching `scribe.svelte`, `scribe-processing.svelte`, or anything that navigates to or opens the Scribe screen.

---

## Architecture

Scribe lives in the main app shell (`capture.svelte` route inside `app-shell.svelte`). Tray/hotkey opens the shell and navigates to Scribe via `shell://navigate` — this must **not** call `scribe_start`. The user starts capture with **Start Recording** in `scribe.svelte`.

---

## Rules — do not regress

- **Never** auto-invoke `scribe_start` when navigating to the Scribe route or focusing the main window.
- **Never** reintroduce `autoStart` / `autoStartRecording` / `scribe://open-requested` arming for recording.
- **Record again** (from the processing screen) returns to the idle Scribe UI. The user taps **Start Recording** again. The error-state **Try again** button in `scribe.svelte` may call `startRecording` directly — this is the only allowed exception.
- Scribe state machine: `IDLE → RECORDING → TRANSCRIBING → DONE | NO_MODEL | ERROR`. The panel is prewarmed at startup but always starts in `IDLE`.
