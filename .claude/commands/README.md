# Skills (canonical source)

**Edit skills here only.** Agent runtimes read copies from:

| Destination | Used by |
|-------------|---------|
| `.cursor/skills/` | Cursor |
| `.claude/commands/` | Claude Code |

Those directories are **generated** — edits there are overwritten on the next sync.

## Sync

After any change under `skills/`:

```bash
bash skills/build.sh
```

The PostToolUse hook (`.cursor/hooks/skills-sync.sh`) runs this automatically when you edit files in `skills/`, `CLAUDE.md`, or `AGENTS.md`.

`build.sh` also keeps `AGENTS.md` ↔ `CLAUDE.md` in sync (newer file wins).

## Skills in this repo

| Skill | Purpose |
|-------|---------|
| `ux-principles/` | UX principles, design intent, interaction patterns (markdown progressive disclosure) |
| `design-skill/` | Design tokens + UX playbook legacy (`query.py`) — being superseded by `ux-principles/` |
| `ui-enforcement/` | Typography, color, layout/scroll rules for frontend |
| `ui-taxonomy/` | UI taxonomy reference |
| `commit-curator/` | Git commit / branch hygiene workflows |
| `new-story/` | Create backlog stories |
| `new-adr/` | Create architecture decision records |

## Related (not synced by build.sh)

These live outside `skills/` and are edited directly:

- `.cursor/rules/ui-enforcement.mdc` — auto-attached cheat sheet for Cursor
- `.cursor/hooks/ui-enforcement-lib.mjs` — deny patterns + hook cheat sheet

When adding enforceable UI rules, update the skill reference chapter **and** the Cursor rule/hooks if agents need the cheat sheet without opening the skill.
