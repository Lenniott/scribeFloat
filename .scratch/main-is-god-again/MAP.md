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
- [Sequential-loading habits in app startup](./issues/23-sequential-loading-habits-in-app-startup.md) — Model loads already lazy (not the problem); the habit is independent file-I/O setup steps chained sequentially in one `.setup()` closure, all gating tray creation, despite the file already using `tauri::async_runtime::spawn` correctly for the last group of startup work. Spun off [Remove legacy voice purge from startup](./issues/22-remove-legacy-voice-purge-from-startup.md), [Reorder startup sequencing](./issues/24-reorder-startup-sequencing.md), [Cache VAD hash fingerprint](./issues/25-cache-vad-hash-fingerprint.md).
- [Sequential-loading habits in the Dictate flow](./issues/27-dictate-flow-sequential-loading.md) — Whole round trip lives in `dictate.rs`. Same "wait until the last possible moment" preload habit recurs (Whisper preload starts after `Recording` is set, not at key-down); plus blind `sleep(50ms)` polling on main-thread hops, needless clipboard-write-after-history-append ordering, and temp-WAV deletion held until after paste. Spun off [Preload earlier](./issues/30-dictate-preload-earlier.md), [Replace blind sleeps](./issues/31-dictate-replace-blind-sleeps.md), [Reorder clipboard vs history](./issues/32-dictate-reorder-clipboard-history.md), [Delete temp WAV early](./issues/33-dictate-delete-temp-wav-early.md).
- [Sequential-loading habits in the Scribe (Record) flow](./issues/28-scribe-flow-sequential-loading.md) — Good news: live diarization and Whisper preload during capture are **already** correctly backgrounded here. One real recurrence: dual-source (mic + speaker) Whisper passes run strictly sequentially on one thread in `transcribe_capture_with_inference` though neither depends on the other. Spun off [Parallelize dual-source transcription](./issues/34-scribe-parallelize-dual-source-transcription.md) (needs a reentrancy check first).
- [Sequential-loading habits in the Transcribe (Upload) flow](./issues/29-transcribe-flow-sequential-loading.md) — Worst-case recurrence of the startup habit: Whisper model is re-hashed via synchronous SHA-256 on **every batch item**, not just once per launch; Sortformer's model load sits strictly after ASR completes despite no real dependency; batch loop can't start item N+1's decode until item N's full write+journal+emit finishes. Spun off [Parallelize Sortformer load](./issues/35-transcribe-parallelize-sortformer-load.md), [Cache Whisper hash](./issues/36-transcribe-cache-whisper-hash.md), [Prefetch next decode](./issues/37-transcribe-prefetch-next-decode.md).
- [Decide Notes list refresh strategy](./issues/26-notes-list-refresh-strategy.md) — **Debounce** chosen over patch-in-place (human, 2026-07-23): coalesce `item-added`/`item-updated` bursts into one refetch; patch-in-place stays out of scope unless debounce proves insufficient. Spun off [Implement notes refresh debounce](./issues/38-implement-notes-refresh-debounce.md).
- [Implement notes refresh debounce](./issues/38-implement-notes-refresh-debounce.md) — **closed**: new `src/lib/utils/debounce.ts` (200ms trailing) wraps both note listeners in `+layout.svelte`; cleanup cancels pending timer.
- [Dictate Whisper preload earlier](./issues/30-dictate-preload-earlier.md) — **closed**: preload now fires from `dispatch_action`'s `Start` branch (key-down/HUD-request time), not gated on mic open inside `start()`.
- [Replace blind main-thread-hop sleeps in Dictate](./issues/31-dictate-replace-blind-sleeps.md) — **closed**: `capture_paste_target_then_open_overlay` is now `async` and awaits a `tokio::sync::oneshot` sent from the main-thread closure instead of `sleep(50ms)`; all three call sites await the real result.
- [Reorder Dictate clipboard-write vs history-append](./issues/32-dictate-reorder-clipboard-history.md) — **closed**: clipboard write (which gates user-visible paste) now runs before history append; kept sequential rather than concurrent since `do_transcription` is already inside one `spawn_blocking` and both writes are cheap sync I/O.
- [Delete Dictate temp WAV earlier](./issues/33-dictate-delete-temp-wav-early.md) — **closed**: deviated from literal "right after `pcm_16k` read" — deleted instead right after transcription succeeds (`segments.is_empty()` check), since two earlier failure branches still need the file for `salvage_dictate_wav`; deleting right after the PCM read would have broken salvage on those paths.

## Not yet specified

- Whether CI on merge (untagged) needs any Silicon-only nuance
- Post-merge: what “slightly better than commodity dictate” must be before a public tag (later map)
- Post-merge wayfinder: Record/Dictate naming honesty (ex-ticket 17; see Known issues)
- Load-performance effort (2026-07-23, "dictate process was slow" → root out the sequential-loading habit wherever it appears): startup, Dictate, Scribe, and Transcribe flows all traced and ticketed (see Decisions so far — tickets 22/24/25, 30-33, 34, 35-37, 38). Two follow-on caching tickets ([25](./issues/25-cache-vad-hash-fingerprint.md) VAD, [36](./issues/36-transcribe-cache-whisper-hash.md) Whisper) look like they may share one implementation — worth checking before building both. Whether Settings/onboarding hide the same pattern is still fog — not yet ticketed. Nothing implemented yet; all tasks are open and unclaimed.
- [Parallelize Scribe dual-source mic/speaker passes](./issues/34-scribe-parallelize-dual-source-transcription.md) — declined, not deferred: `ModelService.inference_gate` already serializes all `whisper_full` calls process-wide because concurrent encode corrupts Metal/ggml state, so `spawn_blocking`-and-join would just queue behind the gate for zero speedup; sequential mic-then-speaker stays as-is, no code changed.

## Out of scope

- Tagging a release / publishing Mac downloads on the website
- Retrieval, embeddings, semantic search, context-file extraction / knowledge layer
- Redesigning Upload UI beyond not lying about the product
- Building out Notes filters beyond what already exists (name); filter product work later
- Red/blue (or other) long-term branching strategy — decide after `main` is clean
- “Impressive to a respectable developer” full code reorg — only org changes that unblock merge confidence
- Float as a promised product surface (UI tease removed this map; funeral/rewrite later if needed)
- Making this competitive with every free dictate app before merge — differentiation is the *next* map’s problem
