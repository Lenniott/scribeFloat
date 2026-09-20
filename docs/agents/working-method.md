# Working method: how we explore, build, review, and decide

This is the standing process for sessions on this repo, from wherever we are now
onward. It doesn't re-litigate past decisions — it points at the tickets that made
them and states the rule going forward.

## Session classification

Use the mode table in `AGENTS.md` (Exploring / Building / Managing) to decide what
to load. There is no separate "review" mode: a review pass (grilling findings,
sorting merge-blocker vs park, smoke-testing) is either a `wayfinder:grilling`
ticket inside a Managing session, or a `/wayfinder` "chart the map" pass. See
`issue-tracker.md`'s Wayfinding operations for how those sessions are structured.

## Thin-doc rule

The keep-set decided during the "Main is God again" thin-docs cut (closed effort)
is permanent: `CONTEXT.md`, `PRIVACY.md`, `AGENTS.md`, `docs/adr/`, `docs/agents/`,
`docs/assets/`, `docs/README.md`. `docs/ideas/` is the parked dump destination for
closed-effort Known issues — it is **not** Binding and **not** a spec. Do not
implement from it without a wayfinder ticket. Everything that was cut
(`architecture.md`, `action-flows.md`, `components.md`, `engineering/`,
`backlog/`, `explorations/`, `audits/`, `features/`, standalone UI review essays)
does **not** come back without an ADR and explicit human sign-off. Prefer code +
ADRs + `.scratch/` over inventing a replacement essay.

## Known issues

Non-blocking debt found during a review pass (smoke, niggle, grilling sort) goes in
that effort's `.scratch/<effort>/KNOWN-ISSUES.md`. It is disposable while the effort
is live — capture freely, promote to a ticket only when sharp.

When an effort closes (its map's destination is reached and stale branches are
cleaned up), move the whole `KNOWN-ISSUES.md` file as-is into `docs/ideas/`, renamed
to something sensible (e.g. `docs/ideas/<effort-slug>-known-issues.md`). No
curation, no picking-and-choosing what's "worth keeping" — everything moves. It
becomes raw material a future `/wayfinder` "chart the map" session can draw its
Destination from.

## Session bridge (HANDOFF.md)

Every wayfinder effort maintains `.scratch/<effort>/HANDOFF.md` alongside its
`MAP.md`. `MAP.md` is long-term memory for the whole effort (destination, decisions
so far, frontier, out of scope). `HANDOFF.md` is short-term working memory — what's
true *right now* — so a fresh agent can resume without chat history.

Every session inside that effort — whether it resolved a wayfinder ticket or did
other work — updates `HANDOFF.md` before ending: what's next on the frontier,
what's dirty in the working tree, what closed, and what the next agent must not
re-discover or re-litigate. `/wayfinder` itself doesn't produce this file (its loop
ends at "record on the map, stop") — maintaining `HANDOFF.md` is this repo's
addition on top of it.

Non-wayfinder sessions (a self-contained bug fix, no map involved) don't need a
`HANDOFF.md` — there's no effort folder to bridge from.

## When to write an ADR

School 1, agreed during the "Main is God again" ADR-reality pass (closed effort):
ADRs capture durable or architectural decisions, binding or aspirational, each
stamped with a Status (Binding / Aspirational / Superseded) and a Wayfinder
provenance line. Never deleted, only amended or re-stamped.

Not every ticket needs one. Write an ADR when a future agent, reading the code
without this conversation, would otherwise wrongly assume something is either
already built or still undecided. If the decision only matters for the life of one
ticket, it belongs in that ticket's resolution, not `docs/adr/`.

## Merge-blocker vs park

Default rubric agreed during the "Main is God again" effort (closed):
human unease + real evidence ⇒ merge-blocker. Everything else parks in that
effort's Known issues. "Just get it done" without sorting is not a resolution.

Apply this whenever a review pass (security review, architecture review, smoke
walk, niggle pass) surfaces a pile of findings that need bucketing before work
continues.

## Public tag is a separate effort

Tagging a release and publishing downloads is not part of a merge-to-main effort by
default. It has its own tooling (`npm run bump`, `scripts/ci-macos-release-build.sh`)
and, if it needs decisions made, its own future `/wayfinder` map — not a step
bundled into this one.
