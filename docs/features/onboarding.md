# Onboarding Workflow

## Purpose

First-run setup wizard that gets new users functional in five designer-approved steps: install a model, grant permissions, and try Dictate before the main UI is shown. The flow intentionally does not collect setup-personalisation answers; detailed configuration lives in Settings. Can be restarted from Settings → General → Restart Setup Wizard.

## Step Map

| # | Step | Purpose |
|---|------|---------|
| 1 | Welcome | Brand moment; escape hatch to Settings |
| 2 | Model Download | Download a Whisper model (skipped if one is already installed) |
| 3 | Permissions | Grant mic (required) and accessibility / input monitoring (optional) |
| 4 | Dictate Practice | Live dictation test with NoteComposer; auto-enter toggle |
| 5 | Feature Tour | Stylised menu-bar graphic; four feature callouts; login-item instructions |

Steps 2–5 show a 4-dot progress indicator. Step 1 (Welcome) has no dots.

Step 2 is skipped automatically if any model is already downloaded (`skipModelStep` flag set in `onboarding.svelte` `onMount`). When skipped, `onboarding.svelte` auto-selects the first downloaded model so Dictate has a model ready.

## Architecture

### Window

- Label: `onboarding`
- Size: 680 × 560, non-resizable, centered
- URL: `/?view=onboarding`
- Listed in `capabilities/default.json` — required for all IPC calls from this window
- Close behavior: destroys the window (unlike other windows which hide). The frontend calls `settings_complete_onboarding` before closing.

### State

`onboarding_complete: bool` in `Config` (defaults `false`). Owned by `ConfigService`, accessed via `SettingsController`. Three commands: `settings_onboarding_status`, `settings_complete_onboarding`, `settings_reset_onboarding`.

`onboarding.svelte` owns step state (`currentStep`, `skipModelStep`). No `OnboardingAnswers` — the 5-step flow collects no personalization answers.

### First-run logic

`lib.rs` checks `!settings_ctrl.is_onboarding_complete()` at startup and calls `open_onboarding_window`. The window is not created if onboarding is already complete.

### Dock icon

The tray-only app hides the Dock icon via `sync_activation_policy`. This must be called in `WindowEvent::Destroyed` for the onboarding window (not `CloseRequested`, where `is_visible()` still returns `true`). See `lib.rs::on_window_event`.

## Model Download Step (Step 2)

- Lists all models from `model_list` with size and download button
- Progress tracked via `model://download-progress` events (direct `listen`, not `createModelDownloadStore`)
- Polls `model_list` every 2 s as a fallback
- Continue button appears once any model is `downloaded`; calls `model_select` on the first downloaded model before advancing
- `model_select` failure surfaces in the UI error slot (does not silently advance)
- Skip button hidden while a download is active

## Permissions Step (Step 3)

- Polls `settings_permissions_status` every 5 s and on window focus
- Grant buttons visible when `can_request` is true
- Mic is required; Accessibility and Input Monitoring are optional
- Continue is disabled until mic permission is granted; optional permission state does not block progress

## Dictate Practice Step (Step 4)

- NoteComposer auto-focused on mount (correct paste target for `dictate_auto_paste`)
- Listens to all `dictate://state-changed` events; tracks `dictateState` for live feedback
- Status indicator (pulsing dot + label) shown during RECORDING / TRANSCRIBING / PASTING
- NoteComposer kept mounted behind `hidden` class during active state — preserves manual draft text
- DONE: sets `noteDraft`; if `autoEnter` on, immediately calls `addNote()`
- ERROR: surfaces `e.payload.error` inline
- TRANSCRIBING → IDLE (empty segments): shows "Nothing was heard" hint

## Feature Tour Step (Step 5)

- Rendered macOS / Windows menu bar with live time, Wifi icon, app icon (`/icon.ico` on macOS, `/favicon.png` on Windows)
- Four feature rows: Scribe, Transcribe, History, Settings
- Platform-conditional login-item instructions (macOS: System Settings → General → Login Items; Windows: Settings → Apps → Startup)
- Single Done button → `settings_complete_onboarding` + `getCurrentWindow().close()`

## Skip and Restart

**Skip to Settings (Welcome step):** `settings_complete_onboarding` → `settings_show_window` → close.

**Restart:** Settings → General → Restart Setup Wizard → `settings_reset_onboarding` → `settings_show_onboarding_window`.

## Design Tokens Used

`bg-panel` (window), `bg-card` (instruction cards), `bg-fill` (status indicator), `bg-brand` / `animate-pulse` (recording dot and step progress), `text-success` (downloaded checkmark), `text-destructive` (errors).

Typography: `sf-headline-sm` (step titles), `text-body-md` (body), `text-label-sm tracking-stamped uppercase` (section labels).
