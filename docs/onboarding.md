# Onboarding Workflow

## Purpose

First-run setup wizard that guides new users through permissions, model installation, and configuration without exposing technical choices. Replaces the previous behavior of opening Settings and immediately marking onboarding complete.

## Step Map

| # | Step | Purpose |
|---|------|---------|
| 1 | Welcome | Brand moment; escape hatch to Settings |
| 2 | Permissions | Grant mic (required) and accessibility (optional) |
| 3 | Quick Setup | 3 guided questions auto-configure speaker capture and model choice |
| 4 | Install Model | Download recommended Whisper model with progress |
| 5 | Scribe Setup | Speaker capture configuration, save folder info |
| 6 | Dictate Setup | Hotkey, auto-paste, auto-enter toggles |
| 7 | History & Output | Markdown export toggle, History explanation |
| 8 | Complete | Summary chips, quick-start tips |

Steps 2–8 show a 7-dot progress indicator. Step 1 (Welcome) has no dots.

## Architecture

### Window

- Label: `onboarding`
- Size: 680 × 560, non-resizable, centered
- URL: `/?view=onboarding`
- Close behavior: destroys the window (unlike other windows which hide). The frontend calls `settings_complete_onboarding` before closing.

### State

`onboarding_complete: bool` in `Config` (defaults `false`). Owned by `ConfigService`, accessed via `SettingsController`. Three commands: `settings_onboarding_status`, `settings_complete_onboarding`, `settings_reset_onboarding`.

### First-run logic

`lib.rs` checks `!settings_ctrl.is_onboarding_complete()` at startup and calls `open_onboarding_window`. The window is not created if onboarding is already complete (return users see only the tray).

### Answers state

`OnboardingAnswers` (in `src/lib/types.ts`) is owned by `onboarding.svelte` and passed down to steps. Each step that modifies answers calls `onNext(updates: Partial<OnboardingAnswers>)`. The orchestrator merges updates and advances the step counter.

## Speaker Capture Platform Matrix

| Platform | Mechanism | Setup required | Onboarding UX |
|----------|-----------|----------------|---------------|
| macOS | BlackHole virtual device | Install BlackHole 2ch | ScribePracticeStep detects via `settings_blackhole_detected`; shows warning with install guidance if absent |
| Windows | WASAPI loopback | None | "Works automatically" confirmation copy |

Detection gate: `settings_speaker_capture_requires_device_name()` returns `true` on macOS only. Windows takes the auto path.

## Model Recommendations

Based on answers from QuestionsStep:

| preferAccuracy | Recommended |
|----------------|-------------|
| true           | `small-en-q5` (~460 MB) |
| false          | `base-en-q5` (~145 MB) |

The recommendation is derived in `ModelStep` from `answers.mainUse` and `answers.preferAccuracy`. Users can override by expanding "Choose a different model".

## Skipping Onboarding

Two paths to skip:
1. **Welcome step** — "Skip to Settings": calls `settings_complete_onboarding`, opens Settings window, closes onboarding.
2. **Complete step** — "Open Settings": same flow.

Both mark onboarding complete so the wizard doesn't re-appear.

## Restarting Onboarding

Settings → Help → "Restart Setup Wizard":
1. Calls `settings_reset_onboarding` (sets flag to `false`)
2. Calls `settings_show_onboarding_window`

The next app launch will also show onboarding since `onboarding_complete` is `false`.

## Design System

Container uses `surfaces.onboarding` spec: `flex flex-col items-center justify-center h-full p-6 gap-0 bg-panel`.

Colors from design tokens: `bg-card` for content cards, `bg-brand` for progress dots, `text-success` for check icons, `text-warning` for alerts.

Typography: `sf-headline-sm` for step titles, `text-body-md` for body, `font-mono text-label-sm tracking-stamped uppercase` for section labels.
