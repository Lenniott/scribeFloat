---
title: Silicon ship-bar smoke
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-agent
blocked_by:
  - "07-remove-float-coming-soon-ui.md"
  - "08-delete-dead-multi-model-paths.md"
  - "12-bundle-only-models-no-runtime-fetch.md"
  - "13-sanitize-transcript-html.md"
  - "14-always-delete-legacy-voice-keychain-key.md"
  - "15-verify-all-bundled-models-before-load.md"
  - "16-least-privilege-ipc-per-window.md"
  - "18-mark-and-amend-adrs-for-reality.md"
parent: MAP.md
---

Ticket **17** was parked to Known issues (2026-07-19) and is **not** a smoke blocker.

## Question

On Apple Silicon, does a cold-ish run clear the ship bar: first-run/permissions → Dictate once → Record once → note in Notes with transcript → speaker rename cascades for that speaker → relaunch still shows the note?

**Done when:** Pass/fail recorded; failures either fixed as merge-blockers or explicitly parked in Known issues with human OK.

## Spec (to-spec)

### Problem Statement

This map’s destination is an untagged merge of the spine into `main` with confidence that capture + Notes still work as a product on Apple Silicon. Unit tests and closed merge-blockers do not prove that. Without a live cold-ish walk — permissions, Dictate, Record, Notes, speaker rename, relaunch — we cannot honestly say the ship bar is clear, and we risk merging a spine that fails the first human session.

### Solution

Run a guided human + agent product walk on Apple Silicon against a cold-ish launch with real bundled models. Record pass/fail per ship-bar step on this ticket. Optional observations (satellite windows after ACL, no Settings → Models, offline) are noted but do not block the bar unless they break a required step. Failures become new merge-blocker tickets (claim → `/to-spec` → human OK → implement) or are parked in Known issues with human OK. This ticket does not ship code.

### User Stories

1. As a first-run user on Apple Silicon, I want onboarding (or a Settings reset of onboarding) to request mic permission, so that capture can start legally and visibly.
2. As a Dictate user who pastes into another app, I want optional Accessibility / Input Monitoring prompts when needed, so that paste is not a silent failure I cannot diagnose.
3. As a Dictate user, I want one hotkey → speak → release cycle to produce text on paste or clipboard, so that quick capture still works after the IPC capability split.
4. As a Dictate user, I want that cycle to create a quick Note visible in Notes / history, so that Dictate is not clipboard-only amnesia.
5. As a Record user, I want New Note / Record → stop to attach a transcript Source to a Note, so that long-form capture still lands in the Note model.
6. As a Record user with Sortformer available, I want anonymous speaker labels on turns, so that diarization is visibly alive (not voiceprint identity).
7. As a Notes browser, I want the Record Note to appear in the list after capture, so that history and Notes agree with what just happened.
8. As a Note reader, I want opening that Note to show sane transcript content, so that the unified editor path is not broken.
9. As a security-conscious reader, I want pasting weird markdown / XSS-ish text into written content (optional poke) to not execute junk in the UI, so that ticket 13’s scrub still holds in the live app.
10. As a Notes user, I want renaming a speaker on that Note to update all turns for that speaker in the Note, so that rename cascade is proven before merge (Known issues called this out).
11. As a returning user, I want quitting fully and relaunching to still show the same Note, so that disk persistence survived the session.
12. As a Silicon tester, I want a cold-ish quit → relaunch before the walk, so that new `dictate` / `onboarding` / `shell` capabilities actually load (old flat default is gone).
13. As a Silicon tester, I want real Whisper / VAD / Sortformer files present (not empty placeholders), so that a “full pass” is not a fake pass that skipped seeding.
14. As a Dictate overlay user, I want the satellite window still usable after least-privilege IPC, so that ACL work did not orphan Dictate UI (optional observation).
15. As an onboarding user, I want the onboarding window still able to complete setup after ACL, so that first-run is not broken by deny-by-default (optional observation).
16. As a Settings user, I want no Settings → Models chooser and errors that say reinstall / bundled models (not “download”), so that single-model honesty holds in the live UI (optional observation).
17. As an offline user, I want no surprise network fetch for models during smoke, so that bundle-only behaviour matches PRIVACY claims (optional observation).
18. As a map owner, I want each required step marked pass or fail on this ticket, so that Resolution is evidence not vibes.
19. As a map owner, I want a failing required step turned into a merge-blocker ticket or explicitly parked with my OK, so that “just get it done” cannot close the bar.
20. As a map owner, I want Record vs Dictate naming debt (ex-ticket 17) ignored as a smoke failure, so that Scribe wording does not block untagged merge.
21. As a map owner, I want Intel / Windows platforms out of this walk’s confidence claim, so that Apple Silicon remains this map’s bar.
22. As the next agent after smoke, I want HANDOFF and map Decisions updated with the gist, so that the session bridge stays truthful.
23. As the next agent after a full pass, I want a clear handoff to Write the forward working method → Merge spine into main untagged → Delete stale branches, so that smoke is the last confidence gate before process + merge tickets.
24. As a human walking the app, I want the agent to ask one checklist question at a time when something fails, so that triage stays plain-language and calm.
25. As a human, I want the agent not to commit or push unless I ask, so that smoke evidence stays separate from tree mutation.
26. As a security reviewer, I want voiceprint never re-opened during smoke (“users who had voiceprints”), so that local hygiene stays closed.
27. As a Record user without Sortformer usable, I want the walk to still pass on transcript + Note persistence, with missing labels noted rather than invented as voiceprint failure.
28. As a Notes user exercising speaker rename, I want awkward edge cases that break cascade promoted only if they really break — otherwise leave Known issues as-is.

### Implementation Decisions

- **This ticket is execution, not implementation.** No product code changes land under ticket 09. Fixes are new tickets (or Known issues parks).
- **Primary seam:** one human-driven cold-ish product walk on Apple Silicon. No new automated smoke harness. Existing unit/ACL tests may be re-run as a precondition sanity check but do not replace the walk.
- **Platform:** Apple Silicon only for this map’s confidence bar.
- **Preconditions before step 1:**
  - Quit the app fully and relaunch so IPC capabilities (`dictate` / `onboarding` / `shell`) load.
  - Prefer real bundled model files in resources; empty placeholders skip seeding and cannot claim a full pass.
  - Dev launch (`npm run tauri dev` / usual Silicon run) is fine if models are real.
- **Required ship-bar steps (must pass or be dispositioned):**
  1. First-run / permissions — onboarding or Settings reset → mic (+ Accessibility / Input Monitoring if Dictate paste needs them).
  2. Dictate once — hotkey → speak → release → paste or clipboard; quick Note appears.
  3. Record once — New Note / Record → stop → transcript on Note; speaker labels if Sortformer available.
  4. Notes — Note visible; open it; content looks sane.
  5. Speaker rename — rename a speaker; that speaker’s turns update.
  6. Relaunch — quit → open → same Note still there.
- **Optional observations** (record notes; not bar blockers unless they break a required step): Dictate/onboarding satellites after ACL; no Settings → Models; offline / no model network fetch.
- **Failure ritual:** claim a new merge-blocker → `/to-spec` → human OK → implement; or park in Known issues with human OK. Do not code a new blocker cold from smoke chat.
- **Naming:** product language Record / Dictate / Note / Source / Transcript. Code may still say Scribe — that is Known issues, not a smoke fail.
- **Voiceprint:** closed topic; do not invent released-user scenarios.
- **After smoke:** append `## Resolution` with pass/fail table; close if pass (or human OK with parked fails); update map Decisions gist + HANDOFF.

### Testing Decisions

- A good “test” here is external behaviour a user can see: permission prompts, pasted text, Note list, transcript body, renamed labels, persistence after relaunch — not controller internals or IPC allowlist unit details (those already closed under ticket 16).
- Module under test: the shipped App on Apple Silicon as one composition (capture → Note → rename → relaunch).
- Prior art: HANDOFF ship-bar checklist; map Notes “niggle pass” complements but does not replace this bar; Known issues “Speaker rename edge cases” explicitly deferred proof to this walk; ticket 16 ACL tests remain compile-time evidence, not this walk.
- Record results as a pass/fail table in `## Resolution` on this ticket. Optional XSS poke is a note, not a required fail if skipped.
- Hardware-gated `#[ignore]` cargo tests are not a substitute for this walk and need not be run for smoke to pass.

### Out of Scope

- Writing or changing product code under this ticket
- Automated E2E / Playwright / Tauri driver smoke suite
- Intel Mac or Windows confidence
- Ticket 17 Record/Dictate naming honesty
- Release tag, website publish, knowledge/embeddings/retrieval
- Upload redesign beyond not lying about models
- Re-litigating voiceprint
- Commit / push unless the human asks
- Blocking on optional observations when required steps pass

### Further Notes

- HANDOFF previously said smoke needs no `/to-spec`; this spec exists because the human asked for a ready-for-agent runbook. Prefer this ticket + HANDOFF over chat memory.
- Refer to this ticket by title: **Silicon ship-bar smoke**.
- Next after close: [Write the forward working method](./02-write-forward-working-method.md) → [Merge spine into main untagged](./10-merge-spine-into-main-untagged.md) → [Delete stale branches](./11-delete-stale-branches.md).

## Comments

### 2026-07-19 — mid-smoke pause (human done for day)

- Spec published; ticket claimed. Prefer **installed** Silicon `.app` for TCC honesty.
- Uncommitted fixes found during smoke: CLI→`scribefloat-cli` (dead DMG), Sortformer startup SHA deferred (tray hang).
- Human OK park → Known issues: early Input Monitoring / Documents prompts; cold Whisper on onboarding Try Dictate.
- Checklist: permissions/Dictate partly seen; **Record / Notes / rename / relaunch still open**. Resume via HANDOFF.

### 2026-07-21 — cold onboarding re-walk; findings triaged

- Cleared app support + relaunched installed `.app` through Setup.
- **Elevated merge-blockers:** [Persist onboarding step across quit](./19-persist-onboarding-step-across-quit.md) (quit for Mic/Input Monitoring → welcome again); [Onboarding Try Dictate Continue reachable](./20-onboarding-try-dictate-continue-reachable.md) (long Dictate buries Continue; opaque multi-send gate).
- **Parked Known issues:** Keystroke dialog under Setup (early TCC); nonsense practice timestamps; gamify double-tap + tap-and-hold; stale “You’re All Set” tray mockup vs live menu.
- Human reached Done / live tray. **Still open for bar:** Record once → Notes → speaker rename → relaunch.

### 2026-07-21 — Record / Notes / Spaces findings

- **Record once: pass** (transcript + speaker labels).
- **Elevated:** [Deleting note text inserts Selection deleted](./21-deleting-note-text-inserts-selection-deleted.md).
- **Parked Known issues:** written pane height; speaker rename this-label vs all; Dictate overlay flaky in full-screen (capture still works); tray “Open” lands on full-screen Space; Record button new-note vs in-note; focus rings hidden by styling.
- **Still open for bar:** relaunch persistence only (speaker rename cascade **pass**; this-vs-all stays Known issues).

## Resolution

Silicon ship-bar walk finished 2026-07-21 on installed `.app` (+ persistence re-checked in `tauri dev`). File-backed Notes/`history.jsonl` — relaunch is “did we write disks,” not a separate DB.

| Step | Result |
|------|--------|
| Preflight / real models | Pass |
| First-run / permissions | Fail → [Persist onboarding step across quit](./19-persist-onboarding-step-across-quit.md); early TCC parked Known issues |
| Dictate once | Pass on capture (double-tap + hold); onboarding Continue trap → [Onboarding Try Dictate Continue reachable](./20-onboarding-try-dictate-continue-reachable.md); overlay Spaces / timestamps / gamify / tray mockup → Known issues |
| Record once | Pass |
| Notes | Open/read pass; delete corrupts body → [Deleting note text inserts Selection deleted](./21-deleting-note-text-inserts-selection-deleted.md); layout/focus → Known issues |
| Speaker rename | Pass (cascade); this-vs-all → Known issues |
| Relaunch | Pass (notes still present after quit/reopen; same on disk in dev) |

Untagged merge still blocked by **19** / **20** / **21** (and forward-method ticket **02**). Smoke itself is closed.
