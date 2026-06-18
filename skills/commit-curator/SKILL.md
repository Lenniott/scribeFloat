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
| **2 — Execute** | User approves (`proceed`, `go`, `lgtm`, etc.) | Stage → commit → verify (backup only when required — see below) |

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

## Same-session fast path (no backup)

Skip the backup snapshot when **all** of these are true:

- You implemented the changes in this conversation (or the user is committing that same work).
- Dirty worktree only — no branch rewrite, rebase, or commit splitting across existing SHAs.
- A single straightforward commit (or a small approved split you just planned).

Turn 2 then: stage → commit → confirm clean worktree. No `backup/*` branch, no tree-hash comparison.

Say in the Turn 1 plan: **Backup: skipped (same-session commit).**

## Non-Negotiables

- **Turn 1 = plan only.** No backup commits, no `git add`, no `git commit` until the user approves the plan in a follow-up message.
- Do not push.
- Do not delete backup branches unless the user asks.
- Do not rewrite history, reset, or commit until the user approves the proposed plan.
- On Turn 2: create a backup snapshot **only when the fast path does not apply** (rewrite branch, unfamiliar/mixed worktree, multi-commit surgery). See `references/backup-and-verification.md`.
- When backup is used, final verification must compare the resulting tree to the backup snapshot tree.

## Turn 1 checklist

Before sending the plan, confirm:

- [ ] Every proposed commit has files, rationale, and full message
- [ ] Backup strategy stated (snapshot ref **or** "skipped — same-session commit")
- [ ] Response ends with **Awaiting approval**
- [ ] No write git commands were run in this turn

## Turn 2 checklist

After user approval:

- [ ] Backup snapshot created **only if** fast path does not apply (ref recorded)
- [ ] Commits match the approved plan (or user was asked about deviations)
- [ ] If backup was used: `git rev-parse backup-ref^{tree}` equals `git rev-parse HEAD^{tree}`
- [ ] Worktree clean; report SHAs and `git log --oneline` for new commits
