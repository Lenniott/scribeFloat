---
labels: [wayfinder:map]
---

## Destination

Every item in the archived [known-issues dump](../../docs/ideas/main-is-god-again-known-issues.md) (25 items, from the closed "Main is God again" effort) is triaged into **Now** or **Later**. "Now" items then get built — this effort carries execution, it doesn't stop at a plan. "Later" items are recorded with why, and left for a future effort (their own wayfinder if big enough) rather than actioned here.

## Notes

- Source dump: `docs/ideas/main-is-god-again-known-issues.md` — read the full entry for an item before triaging it; the ticket only carries the title.
- This effort overrides the default "plan, don't do" — Now-triaged items get implemented in this same effort, not handed off.
- Follow [[feedback_code_practices]] (TDD + DRY + SOLID) for any Now item that touches code.
- Triage tickets are independent — no blocking between them; work any in any order, in parallel across sessions.
- A triage ticket resolves with either: **Now** (state the concrete fix, then do it before closing) or **Later** (state why, and whether it needs its own future wayfinder).
- Items that are already effectively resolved (e.g. informational/no-action) triage straight to Later with a one-line "no action needed" — don't force manufactured work.
- once done a ticket prefix z_ to the title file, then give clear breif but none compressed manual test instructions

## Decisions so far

- [Triage: Onboarding Dictate practice pays cold Whisper load](issues/z_02-onboarding-dictate-cold-whisper-load.md) — **Done** already (`bb027de` eager Whisper preload at startup)
- [Triage: TCC prompts fire too early](issues/z_01-tcc-prompts-documents-access.md) — **Now**, done: gated the startup background compaction+scan behind `!is_first_run` (`src-tauri/src/lib.rs:781`) instead of adding a save-folder-picker step — nothing to compact/recover on a fresh install anyway.
- [Triage: Onboarding Try Dictate shows nonsense timestamps](issues/z_03-onboarding-nonsense-timestamps.md) — **Now**, done: `Timestamp.svelte` now renders wall-clock time via `toLocaleTimeString` instead of treating epoch ms as elapsed duration.
- [Triage: "You're All Set" tray mockup is stale](issues/z_05-onboarding-tray-mockup-stale.md) — **Now**, done: `FeatureTourStep.svelte` mockup updated to Dictate/New note/Open ScribeFloat/Settings/Quit ScribeFloat with separators; `lib.rs:200` casing fixed to "Quit ScribeFloat".
- [Triage: Record button context ambiguity](issues/z_11-record-button-context-ambiguity.md) — **Now**, done: label-only fix — `TitleBar.svelte` button now matches the tray's own semantics: "New note" outside a note (creates, no auto-record), "Record" inside one (starts capture). Confirmation-modal variant not needed; `scribeAutoStart` removed as dead code.
- [Triage: Focus ring hidden on `.sf-select`](issues/z_12-focus-ring-hidden.md) — **Now**, done: added the same `focus-visible:ring-2 ring-focus` rule `.sf-input` already has (`src/app.css:305`).
- [Triage: Opening transcript output allows any .md path](issues/z_19-open-transcript-arbitrary-md-path.md) — **Now**, done: `TranscribeController::open_output_path` now confines to `config.save_folder`, mirroring `SettingsController::open_transcript`. Design call: confine to `save_folder` (not the per-request output folder) since that folder isn't persisted anywhere retrievable at open time.
- [Triage: Windows file-open / "open with" app path](issues/z_21-windows-open-with-app-path.md) — **Later**: rides along with ticket 19's fix automatically (same upstream confinement, shared platform code path). Windows CI/manual verification remains a separate, larger ask deferred until "Windows care returns."
- [Triage: Onboarding should teach double-tap and tap-and-hold](issues/z_04-onboarding-teach-both-gestures.md) — **Now**, done: `dictate://state-changed` event gained a `gesture` field (`"double_tap"`/`"hold"`) sourced from `DictateStartSource`; `DictatePracticeStep.svelte` teaches both gestures in copy and adds a "Gestures tried" progress card. No Continue-gating added — teaching/crediting both, not forcing both.

## Not yet specified

<!-- fog: nothing yet — all 25 items are already sharp enough to ticket as triage questions -->

## Out of scope

<!-- items ruled beyond this effort's destination, with the closed ticket that ruled them out -->
