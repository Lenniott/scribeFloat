---
title: Persist onboarding step across quit
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-agent
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

Status: ready-for-agent

## Spec (to-spec)

### Problem Statement

During first-run Setup, macOS often forces a quit to finish Microphone or Input Monitoring grants. Today onboarding step lives only in Svelte memory (`currentStep` starts at 1). Disk only stores `onboarding_complete`. After quit and relaunch, Setup always opens on Welcome (“Get Started”), so the user feels stuck in a loop even though they already progressed.

### Solution

Persist the current onboarding step while Setup is incomplete. On launch, if onboarding is not complete, open Setup at the saved step (Welcome / Permissions / Try Dictate / Feature tour). Never mark onboarding complete until the user finishes Done or Skip to Settings.

### User Stories

1. As a first-run user who quit on Grant permissions to flip Mic in System Settings, I want relaunch to open Grant permissions again, so that I am not sent back to Get Started.
2. As a first-run user who quit after Mic was granted but before finishing Setup, I want relaunch to resume at least on Permissions (or the later step I had reached), so that progress is not thrown away.
3. As a first-run user on Try Dictate who quit for any reason, I want relaunch to open Try Dictate, so that I do not re-walk Welcome and Permissions.
4. As a first-run user on the Feature tour who quit, I want relaunch to open the Feature tour, so that Done is one step away.
5. As a first-run user who has never started Setup, I want relaunch to open Welcome, so that cold first-run is unchanged.
6. As a first-run user who taps Done on the Feature tour, I want onboarding marked complete and Setup not to reopen on next launch, so that resume does not trap finished users.
7. As a first-run user who chooses Skip to Settings, I want onboarding marked complete the same way as today, so that skip still exits Setup.
8. As a returning user with an older config that has no step field, I want Setup to load without error (default Welcome / step 1 when incomplete), so that upgrades are safe.
9. As a user who resets onboarding from Settings, I want Setup to start at Welcome again, so that reset clears resume state.
10. As a user granting Accessibility or Input Monitoring (optional), I want quitting mid-Permissions to still resume Permissions, so that optional TCC quits do not reset the wizard.
11. As a Silicon smoke tester, I want quit → grant → relaunch to land past Welcome when I had already left Welcome, so that ship-bar first-run can pass.
12. As a maintainer, I want step persistence to reuse the existing Config / SettingsController path used by `onboarding_complete`, so that we do not invent a second settings store.
13. As an onboarding webview, I want only the existing onboarding IPC surface (plus any minimal get/set for step if needed), so that least-privilege IPC from ticket 16 stays intact.
14. As a user who force-quits during Setup, I want the last written step restored, so that crash/quit is treated like an intentional TCC quit.
15. As a user who advances from Welcome to Permissions, I want the step written before or as they leave Welcome, so that a quit immediately after Get Started still resumes correctly.
16. As a reader of the Resolution, I want the step numbering / phase names documented, so that the next agent does not guess 1–4 mapping.

### Implementation Decisions

- **Primary seam:** `Config` + `SettingsController` / settings IPC (same path as `onboarding_complete`), plus `onboarding.svelte` restore on mount and write on step change.
- **Persist a step index or small phase enum** for the four Setup screens: Welcome (1), Permissions (2), Dictate practice (3), Feature tour (4). Prefer a serde-friendly field with `#[serde(default)]` so old configs load.
- **Write on every step transition** (next / back), not only on quit. Back must update the saved step too.
- **Complete paths unchanged:** Feature tour Done and Welcome Skip to Settings call `settings_complete_onboarding` (or equivalent). Completing may clear or ignore the saved step; incomplete + saved step drives resume.
- **Launch behaviour:** when `onboarding_complete` is false, open the onboarding window and initialize UI from the saved step (clamp to 1–4). When complete, do not open Setup.
- **Reset onboarding** from Settings must reset step to Welcome / 1 as well as `onboarding_complete = false`.
- **Do not** mark complete when persisting step. Early TCC dialog timing stays Known issues.
- **IPC:** if new commands or config fields are exposed to the onboarding window, update `permissions/sets/` + `APP_COMMANDS` per `permissions/README.md`. Prefer piggybacking on existing get/complete/reset if a single config read already returns enough.

### Testing Decisions

- Good tests assert **external behaviour**: incomplete config with saved step N → controller / load reports N; complete onboarding → Setup does not resume mid-wizard; old config without the new field still deserializes; reset returns to incomplete + step 1.
- **Modules:** Rust `Config` / config service / `SettingsController` (extend existing onboarding_complete tests); frontend `onboarding.test.ts` for restore-at-step and that Done/Skip still complete.
- **Prior art:** `onboarding_complete_persists_across_reload` in config service tests; `onboarding_starts_incomplete_and_completes` / reset tests in settings controller; `onboarding.test.ts` orchestration.
- No Playwright required. Manual Silicon check remains: quit on Permissions → relaunch → not Welcome.
- `cargo test -p ScribeFloat`, relevant Vitest, and `cargo clippy -- -D warnings` must pass.

### Out of Scope

- Deferring early Keystroke Receiving / TCC dialog under Setup (Known issues)
- Dictation practice gamification
- Tray mockup honesty on “You’re All Set”
- Redesigning onboarding copy or step count
- Persisting Try Dictate practice notes across quit

### Further Notes

- Evidence: Silicon ship-bar smoke 2026-07-21; ticket 09 Resolution dispositioned this as merge-blocker.
- Root cause confirmed in code: `onboarding.svelte` `currentStep = 1` only; `Config.onboarding_complete` boolean only.

## Resolution

Implemented 2026-07-23. `Config.onboarding_step: u8` (serde default 1) persists Welcome=1 / Permissions=2 / Try Dictate=3 / Feature tour=4 via `settings_get_onboarding_step` / `settings_set_onboarding_step`. Onboarding window restores on mount and writes on every next/back. Complete and reset both clear step to 1. Old configs without the field load at Welcome. ACL: onboarding + main-shell sets updated; autogenerated allow files from build.
---
