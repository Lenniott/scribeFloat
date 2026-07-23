---
title: Write the forward working method
labels: [wayfinder:grilling, done]
status: closed
assignee: claude
blocked_by: ["01-finish-thin-docs-cut.md"]
parent: MAP.md
---

## Question

What short, binding process belongs in `AGENTS.md` + `docs/agents/` for how we explore, build, review, capture Known issues, and ADR from here — without bringing back novel architecture essays?

Must cover: session classification, thin-doc rules, Known issues path, when to write an ADR, merge-blocker vs park, and that public tag is a separate effort.

## Resolution

Wrote `docs/agents/working-method.md`, wired from `AGENTS.md` (stub replaced, doc
table + Agent skills section updated) and from `docs/agents/issue-tracker.md`'s
Wayfinding operations section (new `HANDOFF.md` line).

Grilled conversationally (not via the AskUserQuestion tool — the grilling skill
wants a live back-and-forth). Corrected myself mid-session: `/wayfinder` and
`/handoff` are real global Claude Code skills (`~/.claude/skills/`), not absent
from this repo as I first assumed from searching only the project's local
`skills/` folder.

Settled, beyond the six things the question named:

- **Session classification**: no new "review" mode — a review pass is a
  `wayfinder:grilling` ticket inside Managing, or a `/wayfinder` chart-the-map
  pass. Existing AGENTS.md table stands as-is.
- **Thin-doc rule**: z_01's keep-set/cut-list codified as permanent — cut trees
  don't come back without an ADR + human sign-off.
- **Known issues path**: `.scratch/<effort>/KNOWN-ISSUES.md` stays disposable
  while an effort is live. At close-out the whole file moves as-is (renamed, no
  curation) into `docs/ideas/` — the repo's existing durable idea pool — becoming
  raw material a future `/wayfinder` chart session might turn into a new map.
- **Session bridge**: `/wayfinder`'s own loop has no step for a short-term
  working-memory file — this repo's `HANDOFF.md` convention (MAP.md = long-term,
  HANDOFF.md = right-now) is this repo's addition on top of the skill, made
  standing: every wayfinder effort keeps one, every session updates it before
  ending.
- **When to ADR**: z_18's School 1 stands — write one when a future agent would
  otherwise wrongly assume something is either already built or still undecided.
- **Merge-blocker vs park**: z_06's rubric stands verbatim — human unease + real
  evidence ⇒ blocker; "just get it done" isn't a resolution.
- **Public tag**: confirmed as a separate script/effort (`npm run bump`,
  `scripts/ci-macos-release-build.sh` already exist) — one scope-boundary
  sentence, no process to design.

Files touched: `docs/agents/working-method.md` (new), `AGENTS.md`,
`docs/agents/issue-tracker.md`, this ticket, `MAP.md`, `HANDOFF.md`.
