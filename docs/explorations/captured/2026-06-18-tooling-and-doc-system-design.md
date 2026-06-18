---
status: captured
produces: docs/backlog/active/0001-0012
date: 2026-06-18
produces: backlog stories (see docs/backlog/active/)
---

# Exploration: Tooling and documentation system design

All decisions below are resolved. Stories derived from this session should be added to `docs/backlog/active/`.

---

## Resolved decisions

### Folder structure
- `skills/` at repo root — source of truth for all project-level agent skills
- `docs/` — human-readable documentation (ADRs, backlog, explorations, architecture, action-flows)
- `context/` dissolved — contents move to `skills/` (design-skill, code-practice-skill-builder) or `docs/` (architecture.md, action-flows.md, components.md)
- `CONTEXT.md` stays at root — first thing any agent reads

### Cross-tool sync (skills/build.sh)
Build script responsibilities:
1. Copy `skills/` → `.cursor/skills/`
2. Copy `skills/` → `.claude/` (project-level)
3. Sync `AGENTS.md` ↔ `CLAUDE.md` by mtime — most recently modified wins

Hooks:
- **PostToolUse (Claude Code)** — runs `build.sh` on any write inside `skills/`
- **Pre-commit (git)** — runs `build.sh` as safety net
- **Cursor** — write instructions doc for Cursor to self-configure its hook

### AGENTS.md / CLAUDE.md
- `AGENTS.md` is the universal agent instruction file (Cursor, Codex, any tool)
- `CLAUDE.md` is Claude Code's file
- Whichever is newer wins — build.sh syncs the other to match
- Claude Code reads `CLAUDE.md` (confirmed in docs — does not auto-read `AGENTS.md`)
- `/orient` skill dropped — good `AGENTS.md` already does this job

### Backlog structure
Replace `docs/backlog.md` flat file with:
```
docs/backlog/
  README.md          ← naming conventions, ADR linking, how to use
  active/
    0001-slug.md     ← one file per story
    0002-slug.md
  archived/
    0001-slug.md     ← moved here on completion
```
Each story is its own file — point an agent directly at it, no fuzzy search needed.

### Story and ADR creation
- **Skill-based** (`/new-story`, `/new-adr`) for deliberate creation
- **Session hook** fires once per session when EITHER:
  - 15 turns have passed with no writes to `docs/backlog/active/` or `docs/adr/`
  - Context usage passes 60%
  - Asks: "are there any stories or decisions that need capturing?" — suggests them
  - Fires once, does not repeat if dismissed

### Exploration pruning
- Each exploration has frontmatter: `status: active | captured | stale`
- Session hook that asks about stories/ADRs also flags explorations not linked to any ADR or story after 30 days → sets status to `stale`
- User decides what to do with stale explorations — no auto-delete

### Global vs project skills
- Project-level skills (ui-enforcement, orient candidates, backlog management) → `skills/` → built to both tools
- Global skills (domain-modeling, grilling, tdd, writing-rules) → `~/.claude/skills/` → Claude Code only
- User can manually sync global skills to Cursor global if needed — outside this system's scope

---

## Stories to create

- [ ] Restructure: dissolve `context/`, move contents to `skills/` and `docs/`
- [ ] Create `skills/build.sh` with copy + mtime sync logic
- [ ] Add PostToolUse hook to run `build.sh` on writes to `skills/`
- [ ] Add pre-commit git hook for `build.sh`
- [ ] Write Cursor hook setup instructions doc
- [ ] Rename `CLAUDE.md` → `AGENTS.md`, create `CLAUDE.md` that stays in sync via build.sh
- [ ] Create `docs/backlog/` folder structure with `README.md`
- [ ] Migrate existing `docs/backlog.md` stories to individual files in `docs/backlog/active/`
- [ ] Build `/new-story` skill
- [ ] Build `/new-adr` skill
- [ ] Add session-end hook (15 turns OR 60% context → ask about uncaptured stories/ADRs, flag stale explorations)
- [ ] Add `status:` frontmatter to all existing exploration files
