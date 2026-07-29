---
title: "Triage: Onboarding "You're All Set" tray mockup is stale"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Resolution

**Now, done 2026-07-29.** `FeatureTourStep.svelte`'s mockup rewritten to Dictate / New note / Open ScribeFloat / Settings / Quit ScribeFloat with two separators, matching the live tray exactly; `lib.rs` casing fixed to `"Quit ScribeFloat"`. Mockup rows simplified (icon + label, no per-item description) after a follow-up correction — the original description-heavy version overflowed the step footer; the real fix was proportioning the mockup like the compact native menu, not adding scroll clipping.

## Issue

Onboarding's "You're All Set" step shows a mock tray dropdown (Scribe/Transcribe/History/Settings, 4 items, no separators) that no longer matches the real tray menu (Dictate/New note/Open ScribeFloat/Settings/Quit, 5 items, two separators) — mismatched on labels, count, and structure. Separately, the real menu's Quit item is cased "Quit scribefloat" instead of "Quit ScribeFloat", inconsistent with the rest of the menu.

## Question

Read the "Onboarding "You're All Set" tray mockup is stale" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now** — small, pure copy fix in one onboarding component plus a one-line backend casing fix. Any reason to defer?

## Findings

- Onboarding mockup: `src/lib/ui/4_sections/onboarding/FeatureTourStep.svelte:16-35`, `features` array hardcodes exactly the stale set: `Scribe` ("Transcribe long-form recordings and take notes."), `Transcribe` ("Transcribe pre-recorded audio files."), `History`, `Settings`. Rendered as a mock tray dropdown at `:120-137`.
- Real tray menu: `src-tauri/src/lib.rs:176-218` (`build_tray_menu`) builds, in order: `Dictate` (`:180`), `New note` (`:181-182`, with hotkey accelerator), separator, `Open ScribeFloat` (`:183-189`), `Settings` (`:190-196`), separator, `Quit scribefloat` (`:197-203`).
- Mismatch confirmed on all four fronts: item count/labels (mockup has Scribe/Transcribe/History/Settings; live has Dictate/New note/Open ScribeFloat/Settings/Quit), item count (4 vs 5), no separators shown in mockup vs two in live menu, and no Quit item at all in the mockup.
- Quit casing spot confirmed: `src-tauri/src/lib.rs:200`, `MenuItem::with_id(app, QUIT_MENU_ID, "Quit scribefloat", ...)` — lowercase "scribefloat" — inconsistent with the app-name casing used elsewhere in the same menu, e.g. `"Open ScribeFloat"` at `lib.rs:186`, and with the product name "ScribeFloat" generally.
- A fix would concretely touch: `FeatureTourStep.svelte:16-35` (rewrite `features` array to Dictate/New note/Open ScribeFloat/Settings, plus decide whether to visually represent Quit/separators), and `src-tauri/src/lib.rs:200` (change `"Quit scribefloat"` → `"Quit ScribeFloat"` for casing consistency — trivial, independent one-line fix).
- Size estimate: small. Pure frontend content/copy change in one component plus a one-line backend string fix; no logic or architecture changes.
