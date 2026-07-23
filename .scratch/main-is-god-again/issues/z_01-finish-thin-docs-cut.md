---
title: Finish the thin-docs cut
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by: []
parent: MAP.md
---

## Question

Thin docs are intentional. The working tree already deletes a large set of `docs/**` while `AGENTS.md` still points at ghosts.

What exact keep-set and pointer updates make the spine honest before merge — and what must be committed so `main` inherits that method?

**Done when:** Keep-set decided and applied (at least `CONTEXT.md`, `docs/adr/`, `docs/agents/`, thin `AGENTS.md`); broken pointers gone; deletion/keep committed on the spine (or a clear commit plan recorded in the resolution). This ticket decides and records the cut; execution can be the resolution’s task outcome.

## Resolution

### Keep-set

| Path | Role |
|------|------|
| `CONTEXT.md` | Domain glossary |
| `PRIVACY.md` | Privacy claims |
| `AGENTS.md` | Thin session index + build/skills |
| `CLAUDE.md` | Points at `AGENTS.md` only |
| `docs/adr/` | Decisions |
| `docs/agents/` | Tracker / triage / domain consumption |
| `docs/assets/` | Static assets |
| `docs/README.md` | Thin keep-set index |

### Cut (do not recreate without ADR + human OK)

`docs/architecture.md`, `action-flows.md`, `components.md`, `engineering/`, `backlog/`, `explorations/`, `audits/`, `features/`, UI review essays — already deleted in the working tree; pointers removed from live indexes.

### Applied this session

- Rewrote `AGENTS.md` (no ghost pointer table; session modes load CONTEXT + ADRs + code / `.scratch`)
- Rewrote `docs/README.md`; fixed `CONTEXT.md` + `README.md` ghosts
- Retargeted `skills/new-story` → `.scratch/`; synced copies under `.cursor/skills` + `.claude/skills`
- Session-capture hook treats `docs/adr/`, `docs/agents/`, `.scratch/` as capture writes
- Dropped dead `history-storage.md` comment in `history.rs`

### Deferred to *Write the forward working method*

Full binding process text (merge-blocker vs park, when to ADR, public tag separate). `skills/build.sh` is already missing on this spine — note for that ticket or Known issues, not a recreate of engineering essays.

### Commit plan (not executed — waiting on human)

Single docs/tooling commit on the spine, roughly:

- Deleted: cut `docs/**` trees above
- Modified: `AGENTS.md`, `CONTEXT.md`, `README.md`, `docs/README.md`, `skills/new-story`, `skills/README.md`, session-capture hook + README, `history.rs` comment, `.gitignore` (wav fixtures — already dirty)
- Added: `docs/agents/**`
- Leave `.scratch/` untracked unless/until you want the map in git

## Comments

- 2026-07-19: claimed + resolved by cursor-agent in morning handoff session.
