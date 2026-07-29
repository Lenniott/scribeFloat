---
title: "Triage: Onboarding should teach double-tap and tap-and-hold"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

Onboarding's Dictate practice step only teaches and tracks double-tap activation. Tap-and-hold is a fully implemented, live activation path in the backend state machine (`Toggle`/`HoldImmediateStop`/`HoldWhileHeld`), but onboarding has no copy explaining it, no progress tracking toward it, and no way for the frontend to even tell which gesture triggered a given recording.

## Question

Read the "Onboarding should teach double-tap and tap-and-hold" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now** or **Later**? Medium-sized: needs a backend event-payload change (to surface which gesture started a session) plus new onboarding UI/copy/progress tracking — worth doing now, or does it wait behind the related merge-blocker ticket it touches?

## Findings

- Confirmed: `DictatePracticeStep.svelte` only teaches double-tap. Subtitle at `:84` reads "Double-tap {dictateModifierLabel}, speak, then release." and the "How to use" list (`:91-96`) is: click text area → double-tap modifier → speak 2+s → release/tap to stop. No mention of press-and-hold anywhere in this component.
- Backend gesture detection for both styles already exists and is live in production dictate flow, not onboarding-specific: `src-tauri/src/controllers/dictate.rs` has `DictateStartSource` variants including `Toggle` (double-tap), `HoldImmediateStop`, and `HoldWhileHeld` (`dictate.rs:427-438`), all routed through the same `dispatch_action`. This is the real activation state machine the key listener drives (`ensure_key_listener` wired in `lib.rs:770`), so tap-and-hold detection doesn't need new backend logic — onboarding just needs UI that (a) explains it and (b) observes/celebrates it happening, via the existing `dictate://state-changed` event already consumed in `DictatePracticeStep.svelte:59-76` (state values IDLE/RECORDING/TRANSCRIBING/PASTING/DONE/ERROR) — there's currently no signal distinguishing which gesture (`Toggle` vs `Hold*`) started a given recording, so the event payload likely needs to be extended, or the frontend needs to infer it from press/release timing itself.
- What's missing for the "gamify both gestures" product ask: (1) instructional copy/UI for hold-to-talk (currently absent), (2) a way to track/display progress toward "tried both gestures" (no such state exists in `DictatePracticeStep.svelte` today — it only tracks `notes` count against `MAX_NOTES = 2`), (3) possibly a payload/event change so the frontend can tell double-tap vs hold-to-talk apart to award credit for each.
- A fix would concretely touch: `DictatePracticeStep.svelte` (new copy, per-gesture progress UI/state), and likely `src-tauri/src/controllers/dictate.rs` + the `dictate://state-changed` emit site(s) to include which `DictateStartSource` triggered the session, so onboarding can distinguish and gamify both paths. Also touches the merge-blocker ticket referenced in the known-issues doc (`z_20-onboarding-try-dictate-continue-reachable.md`) since "Continue" gating logic would need to account for two gestures.
- Size estimate: medium. Backend gesture detection already exists (no new state machine needed), but plumbing gesture identity through to the frontend event, plus new onboarding UI/copy/progress-tracking, is more than a trivial UI tweak.
