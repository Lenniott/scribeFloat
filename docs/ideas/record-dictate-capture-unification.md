# Record / Dictate capture unification

Parked from [main-is-god-known-issues](../../.scratch/main-is-god-known-issues/MAP.md) ticket 06. Needs its own wayfinder / branch — not a one-and-done triage fix.

## Summary
- Difficulty: **large**. Two genuinely divergent controllers plus leftover "Scribe" UI strings.
- ADR-0003 already decided Record and Dictate are the same capture capability under different `CaptureProfile` config; the dual-controller split is build-order debt, deferred because mid-pipeline refactor risk is high.
- Two separable slices: (1) cosmetic "Scribe" → "Record" rename in live UI; (2) unify `ScribeController` / `DictateController` behind a shared capture system.

## Why it's parked here
Unification is architectural and risky to the live audio path. Cosmetic rename alone is small but easy to conflate with the big refactor — keep both out of opportunistic triage and chart a dedicated destination when ready.

## Research already done
Grounded findings live in the closed triage ticket:
`.scratch/main-is-god-known-issues/issues/z_06-record-dictate-naming.md`

Key anchors:
- ADR: `docs/adr/0003-scribe-and-dictate-are-capture-profiles.md`
- Controllers: `src-tauri/src/controllers/scribe.rs`, `dictate.rs` (~3.2k lines combined)
- Live "Scribe" UI remnants: notes filter/empty-state/badge/help, onboarding FeatureTour + Welcome labels

## Suggested future destination
1. Peel cosmetic rename if product wants naming honesty without touching controllers.
2. Design a shared capture core parameterized by `CaptureProfile` without breaking Dictate hotkeys or Record diarization/session manifests.
3. Do not widen the controller gap in the meantime (ADR-0003 consequence).
