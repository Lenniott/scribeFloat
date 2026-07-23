---
title: Unify Record and Dictate naming and seams
labels: [wayfinder:task, wontfix]
status: closed
assignee: human
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

What concrete rename / seam plan makes Record vs Dictate honest in code and product language before merge — instead of leaving “Scribe” / dual-controller debt as post-merge polish?

**Done when:** Agreed scope executed (full unify, or a written smaller cut that still removes the worst naming lies); human OK that this meets the elevated merge-blocker bar.

Status: wontfix (this map) — parked to Known issues

## Spec (to-spec)

### Problem Statement

Product language and the glossary say **Record** (long-form) and **Dictate** (quick capture). Much of the shipped UI and Rust surface still says **Scribe** (filters, chips, onboarding, hotkeys, command names, controllers). That lie confuses users and agents, and it was elevated to a merge-blocker (sort A3). Full controller unification is real engineering risk and ADR-0003 already says it is deferred — so “unify” for this map must mean an agreed honesty cut, not necessarily one controller.

### Solution

Execute a **written smaller cut** (recommended) that makes Record vs Dictate honest where users and agents look first: visible UI copy, PRIVACY/user-facing strings, glossary-aligned labels, and the worst IPC/module names if the rename is mechanical and safe. Keep two controllers and shared `CaptureProfile` / post-capture seam. Park full `ScribeController`+`DictateController` merge and `HistoryKind` → `quick`/`origin` schema migration unless the human explicitly chooses the full unify scope.

### User Stories

1. As a Record user, I want the app to say “Record” (not “Scribe”) in Notes filters, chips, and onboarding, so that the product matches what I click in the title bar.
2. As a Dictate user, I want Dictate to keep its name, so that quick capture stays distinct from Record.
3. As a Notes browser, I want filter chips and empty states to say Record / Dictate / Upload, so that triage language matches capture verbs.
4. As a reader of Note cards, I want kind labels to say Record (not Scribe) for long-form mic captures, so that lists are not lying.
5. As a first-run user, I want onboarding feature tour copy to say Record, so that I am not taught a retired name.
6. As a hotkey user, I want “Open Record” (or equivalent) wording instead of “Open Scribe” in errors and tray/accelerator copy users see, so that shortcuts match the product.
7. As a PRIVACY / IT reader, I want Scribe→Record and Transcribe→Upload wording where those docs describe capture, so that compliance text matches the app.
8. As an agent reading CONTEXT, I want code and UI closer to glossary terms, so that I do not “fix” the glossary back to Scribe.
9. As a Record user mid-session, I want behaviour unchanged (durable audio, stop confirm, speaker capture, diarization), so that rename work is not a capture rewrite.
10. As a Dictate user, I want paste / temp-audio behaviour unchanged, so that honesty work does not regress quick capture.
11. As an Upload user, I want Upload to remain the bulk-import verb (Transcribe only as legacy code name if not renamed), so that we do not invent a fourth product name.
12. As a maintainer, I want ADR-0003 respected: do not widen the dual-controller gap; do not pretend full unify shipped if it did not.
13. As a Silicon tester, I want a short checklist: Record start/stop, Dictate hotkey, Notes filter labels — so smoke proves naming without a full rewrite.
14. As an implementer of ticket 16, I want any command renames to update capability allowlists in the same change set (or immediately after), so that least-privilege does not break.
15. As a user with old Notes on disk, I want existing `HistoryKind::Scribe` (or serialized kind strings) to still load, so that rename does not orphan history.
16. As a design-system gallery visitor, I want prototypes either updated or clearly marked non-product if they still say Scribe, so that agents do not copy retired labels into production.
17. As a Settings user, I want speaker-capture / device settings labels to say Record where they currently say Scribe, so that Settings matches capture.
18. As a future unifier, I want a one-line Known issues or Resolution note that full controller merge remains deferred, so that this ticket is not re-litigated as incomplete unify.

### Implementation Decisions

- **Primary seam (recommended smaller cut):** User-visible naming + thin IPC/module rename at the command boundary. Controllers may keep files named `scribe.rs` temporarily if a full symbol rename is too large for one merge; prefer renaming user-facing strings and `invoke` command names the frontend calls. Shared transcription remains `CaptureProfile::{Record, Dictate, Upload}`.
- **Scope options (human picks one):**
  - **(S) Smaller honesty cut — recommended for merge.** User-facing Scribe→Record (UI, onboarding, filters, chips, kindLabel, PRIVACY, hotkey validation strings users/agents see). Keep dual controllers. Keep serialized `HistoryKind` values working (map display only, or serde aliases). Optional mechanical rename of `scribe_*` commands → `record_*` **only if** frontend + capabilities (ticket 16) update in lockstep. Upload may stay `transcribe_*` internally for this cut.
  - **(M) Medium — S + Rust module/controller rename.** `ScribeController`→`RecordController`, commands module rename, event names if cheap; still **no** merge of Dictate into Record.
  - **(F) Full unify.** Single capture controller parameterised by profile; `HistoryKind`→`quick`/`origin` per ADR-0010. **Not recommended before untagged main merge** — high regression risk; conflicts with ADR-0003 “deferred”; overlaps ticket 18’s “mark aspirational” story for 0010.
- **Do not implement ADR-0010 schema migration under (S)/(M).** That stays aspirational until a later effort; ticket 18 marks it.
- **Do not reintroduce fast/refined model tiers** while renaming — one bundled Whisper Small (tickets 08/12).
- **Design-system `/design-system` prototypes:** Update obvious Scribe labels or mark as historical mock; not a product surface, but stop teaching agents the wrong name.
- **Coordination with 16:** If command strings change, capability allowlists change in the same PR/session.
- **Coordination with 18:** 18 amends ADR prose; 17 changes running product/code. Avoid duplicate essay rewrites in both.

### Testing Decisions

- Test **external behaviour**: UI label helpers return Record; fixtures/tests that assert chip “Scribe” update; hotkey conflict messages say Record; existing history fixtures with kind `scribe` still render.
- Do not require hardware Record/Dictate for unit tests; Silicon smoke (ticket 09) covers live capture after this lands.
- Prior art: `NoteCard.test.ts`, `historyFormat`, ipc fixtures `storeScribe` (rename or alias carefully).
- `cargo test -p ScribeFloat`, frontend tests if present for touched files, `cargo clippy -- -D warnings`.

### Out of Scope

- Full Record+Dictate controller merge (unless human picks F)
- `HistoryKind` → `quick`/`origin` persistence migration (unless F)
- Float / triage / knowledge layer
- Rebuilding spoken triggers / replacements (Known issues)
- Website marketing beyond what PRIVACY/in-app already touch
- Linux / mobile

### Further Notes

- Architecture review originally parked dual controllers as Known issues; human elevated A3 to merge-blocker — honesty cut satisfies that bar without pretending F shipped.
- CONTEXT already defines Record vs Dictate; this ticket makes the app catch up.
- Map “Not yet specified” called out ticket 17 scope — **this Open choice closes it.**

### Open choice (human)

Pick **S / M / F**. Spec recommends **S** (or **M** if you want Rust names cleaned in the same pass). **F** only if you explicitly accept deferring merge / smoke risk.

### Done when

1. Chosen scope executed; worst user-facing “Scribe” lies for Record are gone.
2. Dictate and Record behaviour unchanged aside from naming.
3. Old Notes still load.
4. Resolution states which scope was chosen and that full unify remains deferred (unless F).
5. Tests/clippy clean.

## Resolution

**2026-07-19 — demoted from merge-blocker.** Human: deeper than S/M/F; park as Known issue for a later wayfinder (not required before Silicon smoke / untagged main merge). Spec retained below for that future session. Ticket **09** no longer blocked by this file.

