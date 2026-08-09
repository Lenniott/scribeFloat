# Handover — main-is-god-known-issues

Short-term working memory for this effort. Every session working this map reads this file first, and **updates it before ending** — whether or not it closed a ticket. Append your own session's summary at the bottom under "Session log"; don't delete earlier entries, and trim only once they're clearly stale (e.g. superseded by a closed ticket).

## What this effort is

Wayfinder map: [MAP.md](MAP.md). Destination: triage all 25 items from the archived [known-issues dump](../../docs/ideas/main-is-god-again-known-issues.md) into Now/Later, then build the Now items in this same effort (execution is in-scope here — see the map's Notes).

## State as of 2026-08-09

**18/25 closed**, none assigned. This session parked four large/experiment items to `docs/ideas/` and closed #08 as already-fixed.

- Closed earlier: 01–05, 11, 12, 17, 19–22, 25 (builds + no-action Laters).
- **Closed 2026-08-09**:
  - **08** — Done already (written pane height). Fill-parent layout is correct; human confirms not a thing anymore.
  - **06, 09, 14, 15** — **Later**, moved to idea docs (own future wayfinder/branch each):
    - [`docs/ideas/record-dictate-capture-unification.md`](../../docs/ideas/record-dictate-capture-unification.md)
    - [`docs/ideas/dictate-overlay-fullscreen-spaces.md`](../../docs/ideas/dictate-overlay-fullscreen-spaces.md)
    - [`docs/ideas/streaming-dictate-transcription.md`](../../docs/ideas/streaming-dictate-transcription.md)
    - [`docs/ideas/notes-metadata-tags.md`](../../docs/ideas/notes-metadata-tags.md)
- **Still open (7)**: 07, 10, 13, 16, 18, 23, 24 — no Now/Later call yet.

## Remaining tickets, grouped

### User-facing (4)
- **07** — Speaker rename edge cases (can't scope a rename to one occurrence)
- **10** — Opening main window from tray can land on a full-screen Space
- **13** — Dual audio recordings get no real per-speaker diarization
- **24** — Bring back spoken triggers as a narrower Dictate-only feature

### Maintenance / optimization (3)
- **16** — Skills/docs mention a deleted Models screen — likely no-action (grep found nothing live), needs final confirm
- **18** — Upload accepts any OS-readable path with no dialog-scoped confinement (security hardening)
- **23** — No dependency/vulnerability scanning in CI (`cargo audit`, `npm audit`, Dependabot)

## Workflow for the next ticket (confirmed with Benjamin 2026-07-29)

One ticket at a time, in this exact order — do not batch multiple tickets before checkpointing:

1. Pick one open ticket. Read its `## Findings`, make the Now/Later call.
2. **Now**: build the fix (follow [[feedback_code_practices]] — TDD, DRY, SOLID).
3. Test it (unit tests + `cargo check`/`svelte-check`, and a manual check where relevant).
4. **Stop and confirm with Benjamin** before finalizing — do not self-close.
5. Only after confirmation: append `## Resolution`, flip `status: closed`, rename the file with a `z_` prefix, and add one line to the map's **Decisions so far**. (The `z_` rename is the *last* step, done together with the resolution — not applied up front before the work/status is settled.)
6. Commit, then push.
7. Move to the next ticket.

If a ticket is **Later**: state why (and flag if it needs its own future wayfinder), same confirm-before-closing rule applies, then commit/push before moving on.

Update this file's Session log before ending a session, even if nothing closed.

## Session log

- **2026-07-24**: Created the map and all 25 triage tickets from the archived known-issues dump. Ran 10 parallel research agents to ground every ticket in current codebase state (findings appended). No triage decisions made yet — next session should start closing tickets per the steps above.
- **2026-07-25 / 2026-07-29**: Confirmed #02 already closed as Done (eager Whisper preload). Logged provisional priority queue here (HANDOVER = short-term order; MAP Decisions so far = final Now/Later). Next: start closing §1 no-action Laters (17, 20, 22, 25), then §2 likely-Now builds.
- **2026-07-29 (later same day)**: Closed all of §2 (01, 03, 05, 11, 12, 19, 21) as Now and built every fix — see per-ticket `## Resolution` sections and map's Decisions so far for specifics. Two corrections landed mid-session from live user feedback, worth knowing before touching this area again: (1) the tray-mockup fix for #05 first tried `overflow-y-auto` clipping on `StepFrame.svelte` to fix a footer overlap — wrong approach, reverted; the real fix was shrinking the mockup itself to the tray's actual compact proportions. (2) #11's first pass kept an `scribeAutoStart` auto-record-on-new-note behavior — wrong, the tray's own "New note" never auto-records; removed that flag entirely (it's genuinely dead now) so "New note" only creates, "Record" (shown only inside an open note) starts capture. Also: adding the onboarding save-folder step required threading `settings_set_output_path`/`dialog:default` through `permissions/sets/onboarding.toml` + `capabilities/onboarding.json` — this trips `acl_capabilities_test.rs`'s satellite-window deny-list guard, which had to be updated with an explicit, documented onboarding-only exception (dictate-overlay stays fully locked down). Next: §3 (04, 06 naming slice, 07, 08) needs product/design calls or a live UI check before triage — no code exploration done on these yet this session.
- **2026-07-29 (process correction)**: Benjamin flagged two things after the §2 batch above: (1) the remaining 13 tickets should be grouped user-facing vs. maintenance/optimization for planning — done, see "Remaining tickets, grouped" above. (2) The `z_` rename should be the *last* step of closing a ticket (done together with `## Resolution`/`status: closed`), not applied before the work/status settles — and going forward, work one ticket at a time with an explicit confirm-with-Benjamin checkpoint before finalizing/committing, rather than batching several tickets through autonomously like the §2 session did. Replaced the old "Next steps" section with the confirmed workflow — read it before starting the next ticket.
- **2026-07-30**: Closed ticket 04 (onboarding teach both gestures) as **Now**, following the one-ticket-at-a-time workflow with a confirm checkpoint before finalizing. Backend: `dictate://state-changed` event gained a `gesture` field; frontend: `DictatePracticeStep.svelte` teaches and credits both double-tap and hold-to-talk. Full detail in the ticket's `## Resolution`. Also found (and corrected) a stale cross-reference in the ticket's Findings — it pointed to a "merge-blocker" ticket name that doesn't exist; the real underlying fix (Continue button reachability) was already shipped and tested, so it wasn't actually a blocker. Next: pick the next ticket from §3 user-facing (06 naming slice, 07, 08) or continue down the grouped list — none of the remaining 12 have a Now/Later call yet.
- **2026-08-09**: Benjamin call — park experiments/large items to idea docs, close #08 as already-fixed. Closed **08** (Done already). Closed **06, 09, 14, 15** as Later → new idea docs under `docs/ideas/`. Human framed this batch as "done here" for the session; **7 tickets still open** (07, 10, 13, 16, 18, 23, 24) if the effort continues. No commit this session unless asked.
