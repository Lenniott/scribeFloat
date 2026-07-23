---
title: Persist onboarding step across quit
labels: [wayfinder:task, needs-triage]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

## Question

When a first-run user quits (or is forced to quit) mid-onboarding to grant Microphone or Input Monitoring in System Settings, can they reopen into the same onboarding step instead of the welcome page?

**Done when:** Quit → grant TCC → relaunch returns to the permissions (or later) onboarding step they were on — or an equally clear resume — so first-run is not a loop back to “Get Started.”

## Why merge-blocker

Ship-bar step 1 is first-run / permissions. On Apple Silicon, Mic and Input Monitoring grants often require quitting the app. Resetting to page 1 makes honest first-run fail the map’s confidence bar (unease + real finding). Early TCC timing remains Known issues; **resume after quit** does not.

## Seen

Silicon ship-bar smoke on installed `.app` (2026-07-21). Human cleared Application Support for cold onboarding. Keystroke Receiving / Mic grant path → quit → reopen landed on welcome (“Get Started”) instead of Grant Permissions / next step.

## Likely fix direction (not to-spec yet)

Persist onboarding progress (step index / phase) before quitting for TCC; on launch, if onboarding incomplete, open Setup at that step (or next after a successful grant). Do not mark onboarding complete until Done.

## Out of scope here

- Deferring early Keystroke Receiving (Known issues)
- Dictation practice gamification
- Tray mockup honesty
---
