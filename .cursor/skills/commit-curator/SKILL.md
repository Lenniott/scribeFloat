---
name: commit-curator
description: Safely split, rewrite, explain, or review git commits. Use when Codex is asked to commit uncommitted work, clean up messy local commits, restructure a feature branch, produce meaningful commit messages, create a commit-by-commit review map, or package exploratory/prototype work for developer review.
---

# Commit Curator

Use this skill to package code changes into reviewable evidence. Route first; do not load every reference by default.

## Start Here

Inspect state before choosing a workflow:

```bash
git status --short
git branch --show-current
git log --oneline --decorate -20
```

Then choose exactly one workflow:

- **Dirty worktree**: current value is in uncommitted changes. Read `workflows/dirty-worktree.md`.
- **Rewrite branch**: current value is in existing messy commits. Read `workflows/rewrite-branch.md`.
- **Review map**: user wants explanation/review guidance without mutation. Read `workflows/review-map.md`.

Always read:

- `references/backup-and-verification.md`
- `references/commit-message-conventions.md`

Read only when needed:

- `references/staging-strategy.md` when splitting mixed files or many files.
- `references/prototype-vs-production.md` when the work is exploratory, design-led, or intended for handoff.

## Non-Negotiables

- Do not push.
- Do not delete backup branches.
- Do not rewrite history, reset, or commit until the user approves the proposed plan.
- Preserve an exact backup snapshot before mutation.
- Final verification must compare the resulting tree to the backup snapshot tree.

