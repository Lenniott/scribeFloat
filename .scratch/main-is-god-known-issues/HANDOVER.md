# Handover — main-is-god-known-issues

Short-term working memory for this effort. Every session working this map reads this file first, and **updates it before ending** — whether or not it closed a ticket. Append your own session's summary at the bottom under "Session log"; don't delete earlier entries, and trim only once they're clearly stale (e.g. superseded by a closed ticket).

## What this effort is

Wayfinder map: [MAP.md](MAP.md). Destination: triage all 25 items from the archived [known-issues dump](../../docs/ideas/main-is-god-again-known-issues.md) into Now/Later, then build the Now items in this same effort (execution is in-scope here — see the map's Notes).

## State as of 2026-07-29

All 25 tickets in `issues/` exist and are **research-grounded** (each has a `## Findings` section). **1/25 closed**, none assigned.

- Closed: [02](issues/02-onboarding-dictate-cold-whisper-load.md) — **Done** already (`bb027de` eager Whisper preload). Recorded on the map's Decisions so far. Ticket + map edits may still be uncommitted locally.
- Open: the other 24 — none have a Now/Later resolution yet.

Final Now/Later calls land on the map when each ticket closes. The queue below is provisional working order only — re-read the ticket before committing.

## Priority queue (provisional — not decisions)

Work top-down; trim an id from this list when its ticket closes.

### 1. Close as no-action Later (fast)
- 17, 20, 22, 25

### 2. Likely Now (small builds)
- 01, 03, 05, 11, 12, 19, 21

### 3. Needs a call / live check before triage
- 04, 06 (naming slice), 07 — product/design
- 08 — live UI check

### 4. Untagged — read findings first
- 09, 10, 15, 16, 18, 23

### 5. Large → Later + own wayfinder (or map Out of scope)
- 06 (unification slice), 13, 14, 24

## Next steps for whoever picks this up

1. Read this file, then the map ([MAP.md](MAP.md)) — destination, Notes, Decisions so far.
2. Pick the next ticket from the priority queue (or any open ticket — they're independent). Assign it to yourself (fill `assignee` in its frontmatter) before starting.
3. Open the ticket, read its `## Findings` section, and make the Now/Later call the Question asks for:
   - **Now**: state the concrete fix, then build it (follow [[feedback_code_practices]] — TDD, DRY, SOLID). Don't stop at a plan for this effort.
   - **Later**: state why, and flag if it's big enough to need its own future wayfinder (see queue §5).
4. Record the resolution: append `## Resolution` to the ticket, set its `status: closed`, and add one line to the map's **Decisions so far**. Trim the id from the priority queue above.
5. If a Later item turns out to sit past this effort's destination entirely, close it and add a line to the map's **Out of scope** instead of Decisions so far.
6. Update this file before ending your session — one entry in the Session log below, even if you didn't close anything.

## Session log

- **2026-07-24**: Created the map and all 25 triage tickets from the archived known-issues dump. Ran 10 parallel research agents to ground every ticket in current codebase state (findings appended). No triage decisions made yet — next session should start closing tickets per the steps above.
- **2026-07-25 / 2026-07-29**: Confirmed #02 already closed as Done (eager Whisper preload). Logged provisional priority queue here (HANDOVER = short-term order; MAP Decisions so far = final Now/Later). Next: start closing §1 no-action Laters (17, 20, 22, 25), then §2 likely-Now builds.
