# Dictate Activation Save Policy

## Summary
- Difficulty: **medium**. The audio/transcription path does not need major changes, but the Dictate keyboard state machine and settings/config surface both need careful updates.
- Change hold activation from **tap, then hold Control** to **hold Control** after a calibrated threshold.
- Add Settings controls for whether **double-tap** and **hold** Dictate activations create `history.jsonl` Note records.
- Default policy: **double-tap saves**, **hold does not save**.
- Title-bar Dictate button: **never saves**; it still copies/pastes as Dictate does today.

## Key Changes
- Refactor `DictateKeyTracker` so a lone modifier hold enters hold-to-talk after an initial threshold, while double-tap remains toggle-style Dictate.
- Extend the platform modifier listener to treat any non-modifier key press before the hold threshold as a cancellation signal, so normal Control chords do not accidentally start Dictate.
- Carry an internal activation source through the Dictate session: `DoubleTap`, `Hold`, or `ManualButton`.
- In Dictate finalization, only call `HistoryService::append` and emit `note://item-added` when the activation source is configured to save.
- Add config shape with serde defaults, for example `dictate_history_save_triggers: { doubleTap: true, hold: false }`.
- Add Tauri settings commands for reading/writing that policy, register them in `generate_handler!`, `build.rs`, and `main-shell` permissions.
- Update Settings → Dictate with two toggles: “Save double-tap dictations” and “Save hold dictations”.
- Update minimal onboarding/help copy so it no longer implies every Dictate activation always saves.

## Public Interfaces
- New settings API:
  - `settings_get_dictate_history_save_triggers() -> { doubleTap: boolean, hold: boolean }`
  - `settings_set_dictate_history_save_triggers({ triggers })`
- No change to `history.jsonl` record schema.
- No change to Dictate state events unless implementation wants an internal-only diagnostic; skipped history writes should not be reported as `history_write_failed`.

## Test Plan
- Rust unit tests for the key tracker:
  - lone hold crosses threshold and starts hold-to-talk
  - release after hold stops
  - short single Control tap does nothing
  - double-tap still starts toggle Dictate
  - third tap still stops toggle Dictate after cooldown
  - non-modifier key before hold threshold cancels pending hold
- Rust config/settings tests:
  - legacy configs missing the new field load with `doubleTap=true`, `hold=false`
  - setting updates persist across reload
- Dictate controller tests or focused service tests:
  - double-tap source appends to `history.jsonl` when enabled
  - hold source skips append when disabled
  - title-bar/manual source skips append
  - skipped append still writes clipboard/pastes and does not emit `note://item-added`
- Frontend tests:
  - Settings loads both save toggles
  - toggling each invokes the new settings setter
  - save failure reverts UI and shows the existing settings error pattern
- Verification:
  - `cargo test -p ScribeFloat dictate`
  - `cargo test -p ScribeFloat acl_capabilities`
  - relevant Svelte/Vitest settings tests
  - manual macOS smoke: Control hold, double-tap, Control+C/Control+V, and title-bar Dictate

## Assumptions
- Initial hold threshold stays at the current **500 ms** constant for first implementation; calibrate manually after the behavior is live.
- The threshold is not exposed in Settings for v1.
- “Doesn’t save” means no `history.jsonl` line and no new Note, but clipboard/paste behavior remains unchanged.
- Existing Record/Upload history behavior is untouched.
