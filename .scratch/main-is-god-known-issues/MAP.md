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

## Decisions so far

- [Triage: Onboarding Dictate practice pays cold Whisper load](issues/02-onboarding-dictate-cold-whisper-load.md) — **Done** already (`bb027de` eager Whisper preload at startup)

## Not yet specified

<!-- fog: nothing yet — all 25 items are already sharp enough to ticket as triage questions -->

## Out of scope

<!-- items ruled beyond this effort's destination, with the closed ticket that ruled them out -->
