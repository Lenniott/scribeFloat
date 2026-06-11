---
name: commit-curator
description: Safely split, rewrite, explain, or review git commits. Use when asked to commit uncommitted work, split changes, propose commit messages, clean up messy local commits, or curate a branch. ALWAYS present a commit plan and wait for user approval before running git add/commit — even when the user says "commit".
---

# Commit Curator

Use this skill to package code changes into reviewable evidence. Route first; do not load every reference by default.

## ⚠ Two-turn protocol (read this first)

| Turn | When | Do |
|------|------|-----|
| **1 — Plan** | First response to a commit/split request | Inspect diff, write full plan, ask for approval. **No `git commit`.** |
| **2 — Execute** | User approves (`proceed`, `go`, `lgtm`, etc.) | Backup snapshot → stage → commit → verify tree |

**"Commit these changes" means "figure out how to commit" — not "commit now without showing me."**

Full rules: `references/approval-gate.md`

## Start Here

Inspect state before choosing a workflow (read-only):

```bash
git status --short
git branch --show-current
git log --oneline --decorate -20
```

Then choose exactly one workflow:

- **Dirty worktree**: uncommitted changes → `workflows/dirty-worktree.md`
- **Rewrite branch**: messy existing commits → `workflows/rewrite-branch.md`
- **Review map**: explain only, no mutation → `workflows/review-map.md`

Always read before Turn 1:

- `references/approval-gate.md`
- `references/commit-message-conventions.md`

Read before Turn 2:

- `references/backup-and-verification.md`

Read only when needed:

- `references/staging-strategy.md` — mixed files or many files
- `references/prototype-vs-production.md` — exploratory / handoff work

## Non-Negotiables

- **Turn 1 = plan only.** No backup commits, no `git add`, no `git commit` until the user approves the plan in a follow-up message.
- Do not push.
- Do not delete backup branches.
- Do not rewrite history, reset, or commit until the user approves the proposed plan.
- On Turn 2: preserve an exact backup snapshot before the first curated commit.
- Final verification must compare the resulting tree to the backup snapshot tree.

## Turn 1 checklist

Before sending the plan, confirm:

- [ ] Every proposed commit has files, rationale, and full message
- [ ] Backup strategy is named but not created yet
- [ ] Response ends with **Awaiting approval**
- [ ] No write git commands were run in this turn

## Turn 2 checklist

After user approval:

- [ ] Backup snapshot created and ref recorded
- [ ] Commits match the approved plan (or user was asked about deviations)
- [ ] `git rev-parse backup-ref^{tree}` equals `git rev-parse HEAD^{tree}`
- [ ] Report SHAs and `git log --oneline` for new commits
