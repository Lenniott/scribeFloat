---
id: "0013"
title: Guard skill writes — agents must write to skills/, not .cursor/skills/ or .claude/commands/
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Skill write guardrails

Agents default to writing skills wherever they find existing ones. Without a rule and a hook, an agent will write to `.cursor/skills/` or `.claude/commands/` directly, bypassing `skills/` as the source of truth and breaking the build.sh sync model.

## Two-layer fix

### Layer 1 — AGENTS.md rule (helps agents get it right first time)

Add to `AGENTS.md` / `CLAUDE.md` under a "Skills" section:

> **Always write new or updated skills to `skills/`** — never to `.cursor/skills/` or `.claude/commands/` directly. Those directories are managed by `skills/build.sh` and will be overwritten. Run `bash skills/build.sh` after any edit to `skills/` if the PostToolUse hook has not already done so.

### Layer 2 — PreToolUse hook (catches agents that missed the instruction)

Add a PreToolUse hook to `.claude/settings.json` that blocks writes targeting `.cursor/skills/` or `.claude/commands/` and tells the agent to use `skills/` instead:

```json
{
  "type": "command",
  "command": "jq -r '.tool_input.file_path // \"\"' | grep -qE '(\\.cursor/skills/|\\.claude/commands/)' && echo '{\"decision\":\"block\",\"reason\":\"Write to skills/ instead — .cursor/skills/ and .claude/commands/ are managed by build.sh and will be overwritten.\"}' || true"
}
```

Add the equivalent to `.cursor/hooks.json` `preToolUse`.

## Acceptance

- An agent attempting to write `skills/my-skill/SKILL.md` succeeds and `build.sh` syncs it
- An agent attempting to write `.cursor/skills/my-skill/SKILL.md` is blocked with a clear redirect message
- `AGENTS.md` contains the skills write rule
