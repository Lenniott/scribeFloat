---
title: "Triage: Skills / plans still mention deleted Models screen"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Resolution

**Later / no action, closed 2026-08-09.** Premise is already stale. Repo-wide re-grep of `skills/`, `docs/`, and `src/` found no live "Models screen" (or Models route) references — only this ticket's source dump entry in `docs/ideas/main-is-god-again-known-issues.md`. `skills/ui-taxonomy/SKILL.md` Views list has Home + Notes only. Nothing to edit.

**Manual check:** none — no product behavior changed. Optional: `rg -ni 'models screen' skills docs src` should hit only the archived known-issues dump.

## Issue

Ticket claims skills/docs still reference a deleted "Models" screen that no longer exists in the product. A repo-wide grep (skills/, docs/) found no live references anywhere — only the known-issues doc itself and stale `.claude/worktrees/agent-*` artifacts, which aren't part of live docs/skills content.

## Question

Read the "Skills / plans still mention deleted Models screen" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Later** — no action needed, the premise appears already stale/resolved. Does anyone recall a specific stale mention elsewhere (e.g. `.scratch/` plan files) worth a targeted follow-up grep?

## Findings

- Ran `grep -rni "models screen"` (and case variants) across the whole repo, excluding `node_modules`, `target`, `.git`. The only hits are the two triage/known-issues docs themselves:
  - `docs/ideas/main-is-god-again-known-issues.md` (the entry this ticket references)
  - this ticket file (`.scratch/main-is-god-known-issues/issues/16-skills-mention-deleted-models-screen.md`)
  - Two stale copies of the known-issues doc under `.claude/worktrees/agent-*/. scratch/main-is-god-again/KNOWN-ISSUES.md` and `.claude/worktrees/agent-*/context/architecture.md` — these are leftover agent worktree artifacts, not live docs/skills content, and not part of `skills/` or `docs/`.
- Checked `skills/ui-taxonomy/SKILL.md` specifically (the file the original note names as an example) — `grep -ni "models" skills/ui-taxonomy/SKILL.md` returns **zero matches**. The file's "Views" example list (lines 140-141) currently shows only `home.svelte — Home screen` and `notes.svelte — Notes screen`; no Models/Settings-models entry present.
- Also grepped every `skills/*/SKILL.md` for `\bmodels\b` — no matches anywhere in the skills directory.
- Conclusion: the premise is currently stale. Either `skills/ui-taxonomy/SKILL.md` was already edited to remove the Models screen mention since this note was written, or the note's example was inaccurate. As of this investigation, there is nothing to grep-and-fix in `skills/` or `docs/` — no file needs a line removed/updated for this specific issue.
- Size estimate: N/A / zero-effort if confirmed — no live references found. (If a human still recalls a specific stale mention elsewhere, e.g. in `.scratch/` plan files not covered by this grep scope, a follow-up targeted grep of `.scratch/` would be trivial, under 5 minutes.)
