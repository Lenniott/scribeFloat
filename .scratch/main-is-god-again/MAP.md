---
title: Main is God again
labels: [wayfinder:map]
status: open
assignee:
---

# Main is God again

## Destination

`feature/0.3/embeds` (this spine) is merged into `main` **without a release tag**. `main` holds respectable, non-broken, security- and architecture-reviewed code that matches the single-model product; thin working docs describe how we change the project going forward; a **Known issues** list holds non-blocking debt; stale branches are deleted after an approved checklist. Public tagging / website download is explicitly a later effort.

## Notes

- **Domain:** Capture + Notes + anonymous speakers/names. Speakers matter for future context packs (who spoke before what was said); rename cascades across that speaker’s turns in a note.
- **Tracker:** Local markdown — see `docs/agents/issue-tracker.md` → Wayfinding operations.
- **Quality stance:** Human is product conscience and not a hardened engineer; agents must use written rubrics and evidence. Unease + a real finding ⇒ merge-blocker. “Just get it done” is not a resolution.
- **Skills:** `/grilling`, `/domain-modeling`, security-review / architecture review passes, `/research`-style sweeps when resolving research tickets. After the map clears → `/to-spec` (not straight to `/implement` for the whole destination).
- **Platform care:** Apple Silicon is what we exercise for confidence. Intel is not a focus of this map.
- **Known issues:** Capture freely in [`KNOWN-ISSUES.md`](./KNOWN-ISSUES.md); promote to implementation tickets only when sharp.
- **Niggle pass:** Screen/workflow walk with a live Record (or Dictate→note) and spoken narration; share the note with the agent; triage into Known issues vs merge-blockers. Complements *Silicon ship-bar smoke*, does not replace the sort-findings grilling.
- **Session bridge:** Resume via [`HANDOFF.md`](./HANDOFF.md) — do not rely on chat memory.
- **Refer by name:** Always use ticket titles in narration.

## Decisions so far

- **Architecture and single-model review** — In-app chooser UI already removed; merge still blocked by dead selection paths + docs/marketing that sell multi-model download / fast–refined tiers. Inventory for deletion: [research/architecture-single-model-review.md](./research/architecture-single-model-review.md) ([ticket](./issues/05-architecture-single-model-review.md)).
- [ADR reality audit](./issues/03-adr-reality-audit.md) — 8 binding / 5 aspirational / 1 superseded; mark Sources, folders, HistoryKind before agents trust them
- **Security review with rubric** — Findings in [`research/security-review.md`](./research/security-review.md); human sorted in ticket 06.
- [Finish the thin-docs cut](./issues/01-finish-thin-docs-cut.md) — Keep-set applied and committed on the spine; ghost trees stay deleted; work lives in `.scratch/`.
- [Remove Float coming-soon from shipped UI](./issues/07-remove-float-coming-soon-ui.md) — Sidebar tease + `/float` route gone; Home fake Float/Drafts tiles removed; glossary term kept (no funeral).
- [Sort findings into merge-blockers vs Known issues](./issues/06-sort-findings-merge-vs-known.md) — Buckets agreed; Known issues in [`KNOWN-ISSUES.md`](./KNOWN-ISSUES.md); merge-blockers through **16** + **18** closed (**17** demoted); next is Silicon smoke.
- [Delete dead multi-model paths](./issues/08-delete-dead-multi-model-paths.md) — Full collapse: no config selection fields, no catalog-id APIs, no Upload `model_id`; all capture paths use bundled Small; errors say reinstall not Settings → Models.
- [Bundle-only models — no runtime downloads](./issues/12-bundle-only-models-no-runtime-fetch.md) — Startup VAD HF fetch gone; hard offline/reinstall on missing VAD; PRIVACY/README/site/CONTEXT match bundle-only.
- [Sanitize transcript HTML](./issues/13-sanitize-transcript-html.md) — markdown → HTML uses narrow options + ammonia scrub before `{@html}`; XSS payloads stripped.
- **Voiceprint never shipped** — Built only inside this exploration / branch fog on the human’s machine; **never in a public release**. Leftover Keychain key / on-disk purge work is **local hygiene** so `main` looks as if voiceprint never happened — not a multi-user upgrade problem. After ticket **14** (and related purge) closes for this map, **stop re-litigating voiceprint** in chat and new tickets; do not invent “users who had voiceprints” scenarios.
- [Always delete legacy voice Keychain key](./issues/14-always-delete-legacy-voice-keychain-key.md) — Startup always calls Keychain delete (no `profiles_dir_removed` gate); missing key = success. Local hygiene; voiceprint topic closed for this map.
- [Unify Record and Dictate naming and seams](./issues/17-unify-record-dictate-naming-and-seams.md) — **demoted** from merge-blocker to Known issues (2026-07-19); later wayfinder
- [Verify all bundled models before load](./issues/15-verify-all-bundled-models-before-load.md) — Sortformer SHA + offline re-seed from installed app resources; Whisper/VAD heal on integrity fail; no runtime download.

- Ticket **16** aggression **B+** agreed; ticket **18** **School 1** + wayfinder provenance stamps agreed (School 2 too harsh for wayfinder-before-code).
- [Least-privilege IPC per window](./issues/16-least-privilege-ipc-per-window.md) — B+ capability split (`dictate` / `onboarding` / `shell`); AppManifest ACL; static deny-list tests.
- [Mark and amend ADRs for reality](./issues/18-mark-and-amend-adrs-for-reality.md) — School 1 status marks on all 14 ADRs; ADR-0010/0003 amended for one bundled Whisper Small; README index updated; grep clean for multi-model stale prose.
- **Silicon smoke 2026-07-21 onboarding findings sorted** — Merge-blockers: [Persist onboarding step across quit](./issues/19-persist-onboarding-step-across-quit.md), [Onboarding Try Dictate Continue reachable](./issues/20-onboarding-try-dictate-continue-reachable.md). Parked in Known issues: Keystroke dialog under Setup (expand early-TCC), nonsense Try Dictate timestamps, gamify double-tap + tap-and-hold, stale “You’re All Set” tray mockup. Smoke walk (Record → Notes → rename → relaunch) still open on ticket **09**.
- **Silicon smoke 2026-07-21 Notes / Dictate Spaces findings** — Record once **pass**. Merge-blocker: [Deleting note text inserts Selection deleted](./issues/21-deleting-note-text-inserts-selection-deleted.md). Parked Known issues: written pane height; speaker rename this-vs-all; Dictate overlay / main window vs full-screen Spaces; Record button new-note vs in-note; focus ring styling.
- [Silicon ship-bar smoke](./issues/09-silicon-ship-bar-smoke.md) — **closed** 2026-07-21. Pass: preflight, Dictate capture, Record, rename cascade, relaunch (file-backed). Fail dispositioned → **19** / **20** / **21**. Next: `/to-spec` those three → forward method **02** → merge **10**.
- **`/to-spec` for smoke merge-blockers (2026-07-23)** — Specs `ready-for-agent` on [Persist onboarding step across quit](./issues/19-persist-onboarding-step-across-quit.md), [Onboarding Try Dictate Continue reachable](./issues/20-onboarding-try-dictate-continue-reachable.md) (clamp long practice preview with `...`; no Continue gate), [Deleting note text inserts Selection deleted](./issues/21-deleting-note-text-inserts-selection-deleted.md) (CodeMirror `.cm-announced` chrome). Browser repro confirmed 20/21. Next: implement → **02** → merge **10**.
- [Persist onboarding step across quit](./issues/19-persist-onboarding-step-across-quit.md) — `onboarding_step` 1–4 in Config; resume after TCC quit.
- [Onboarding Try Dictate Continue reachable](./issues/20-onboarding-try-dictate-continue-reachable.md) — practice preview **2-line** CSS clamp (`maxLines={2}`).
- [Deleting note text inserts Selection deleted](./issues/21-deleting-note-text-inserts-selection-deleted.md) — MarkdownEditor `.cm-announced` sr-only theme.

## Not yet specified

- Whether CI on merge (untagged) needs any Silicon-only nuance
- Post-merge: what “slightly better than commodity dictate” must be before a public tag (later map)
- Post-merge wayfinder: Record/Dictate naming honesty (ex-ticket 17; see Known issues)

## Out of scope

- Tagging a release / publishing Mac downloads on the website
- Retrieval, embeddings, semantic search, context-file extraction / knowledge layer
- Redesigning Upload UI beyond not lying about the product
- Building out Notes filters beyond what already exists (name); filter product work later
- Red/blue (or other) long-term branching strategy — decide after `main` is clean
- “Impressive to a respectable developer” full code reorg — only org changes that unblock merge confidence
- Float as a promised product surface (UI tease removed this map; funeral/rewrite later if needed)
- Making this competitive with every free dictate app before merge — differentiation is the *next* map’s problem
