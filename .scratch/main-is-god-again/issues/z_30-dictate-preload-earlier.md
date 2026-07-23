---
title: Start Dictate Whisper preload at key-down/HUD-open, not after Recording is set
labels: [wayfinder:task]
status: closed
assignee: claude-agent (worktree agent-ab5cdd260cc85ba7f; hand-reconciled onto release/0.3 by orchestrating session)
blocked_by: []
parent: MAP.md
---

## Question

`spawn_record_start_preload()` in the live Dictate path (`dictate.rs:595, 603-614`) only fires after `start()` has already set state to `Recording` — after the HUD opens and the mic device is resolved. Preload depends only on config (`model.default_model_path()`), not on mic/audio state, so it could start at key-down or HUD-open time instead. This is the same "wait until the last possible moment" pattern already known from the onboarding cold-load Known issue ("Onboarding Dictate practice pays cold Whisper load"), confirmed recurring in the main Dictate path too.

**Done when:**
1. Whisper preload for Dictate kicks off as early as key-down/HUD-request, not gated on mic/device resolution completing first.
2. No change to when recording actually starts from the user's perspective — this is purely about when the background preload thread is spawned.
3. Manual check: a Dictate session immediately after app start (cold model) should show a shorter or absent "waiting for model" delay.
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass; approach recorded in Resolution.

## Comments

- 2026-07-23: Spun off [[27-dictate-flow-sequential-loading]]. Related Known issue: "Onboarding Dictate practice pays cold Whisper load" — same fix likely helps both.

## Resolution

Moved the `spawn_record_start_preload()` call from inside `DictateController::start()` (fired only after mic open + state already `Recording`) to the top of `dispatch_action`'s `DictateAction::Start(source)` branch — the earliest point common to all three start sources (Toggle, HoldWhileHeld, HoldImmediateStop), before the HUD-open main-thread hop or mic device resolution happens. Preload only reads config (`preload_path_for_dictate` + `model.model_available`), so firing it here has no dependency on mic/audio state and doesn't change when recording itself starts.

`start()` no longer calls the preload; `spawn_record_start_preload` is now called once per `Start` dispatch from `dispatch_action`, before `match source`.

**Verify:** `cargo test -p ScribeFloat` → all `controllers::dictate::tests` green, incl. the three `dictate_preload_path_*` eligibility tests (unaffected — pure logic, no timing dependency). `cargo clippy -p ScribeFloat -- -D warnings` clean.
