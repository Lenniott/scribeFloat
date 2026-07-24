# Handover — main-is-god-known-issues

Short-term working memory for this effort. Every session working this map reads this file first, and **updates it before ending** — whether or not it closed a ticket. Append your own session's summary at the bottom under "Session log"; don't delete earlier entries, and trim only once they're clearly stale (e.g. superseded by a closed ticket).

## What this effort is

Wayfinder map: [MAP.md](MAP.md). Destination: triage all 25 items from the archived [known-issues dump](../../docs/ideas/main-is-god-again-known-issues.md) into Now/Later, then build the Now items in this same effort (execution is in-scope here — see the map's Notes).

## State as of 2026-07-24

All 25 tickets in `issues/` exist and are **research-grounded** — each has a `## Findings` section (file:line references, current behavior, fix size estimate) appended by a research pass, but **none have been triaged yet**. No ticket is closed. No ticket is assigned. Nothing has been decided or built.

Rough read of the findings, for orientation only — not a decision, don't skip re-reading the actual ticket:
- Looks like a quick Now: 01, 02, 03, 05, 11, 12, 19, 21
- Needs a product/design call before it can be built: 04, 06 (naming slice only), 07
- Needs a live check, not more code reading: 08
- Confirmed no action needed: 17, 20, 22, 25
- Large — likely wants its own future wayfinder, don't try to squeeze it into this effort: 06 (unification slice), 13, 14, 24

## Next steps for whoever picks this up

1. Read this file, then the map ([MAP.md](MAP.md)) — destination, Notes, Decisions so far.
2. Pick a ticket (any of the 25 in `issues/` — they're independent, no blocking between them, so pick whichever you want to drive to a decision). Assign it to yourself (fill `assignee` in its frontmatter) before starting.
3. Open the ticket, read its `## Findings` section, and make the Now/Later call the Question asks for:
   - **Now**: state the concrete fix, then build it (follow [[feedback_code_practices]] — TDD, DRY, SOLID). Don't stop at a plan for this effort.
   - **Later**: state why, and flag if it's big enough to need its own future wayfinder (some already look that way — see the "Large" list above).
4. Record the resolution: append `## Resolution` to the ticket, set its `status: closed`, and add one line to the map's **Decisions so far**.
5. If a Later item turns out to sit past this effort's destination entirely, close it and add a line to the map's **Out of scope** instead of Decisions so far.
6. Update this file before ending your session — one entry in the Session log below, even if you didn't close anything.

## Session log

- **2026-07-24**: Created the map and all 25 triage tickets from the archived known-issues dump. Ran 10 parallel research agents to ground every ticket in current codebase state (findings appended). No triage decisions made yet — next session should start closing tickets per the steps above.
