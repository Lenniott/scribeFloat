# Backup and Verification

The backup must preserve the exact value the user wants curated — **when a backup is required**.

## When backup is required

| Scenario | Backup |
|----------|--------|
| Branch rewrite / rebase / cherry-pick curation | **Required** — branch at original tip |
| Mixed or unfamiliar dirty worktree (not authored this session) | **Required** — snapshot commit |
| Multi-commit surgery with risk of losing hunks | **Required** |
| Same-session commit: you just implemented the changes, simple dirty worktree | **Skip** — stage and commit directly |

## Same-session fast path (no backup)

When skipping backup, Turn 2 verification is:

```bash
git status --short   # must be clean after last commit
git log --oneline -n <N>
```

No `backup/*` ref. No tree-hash comparison.

## Dirty Worktree Backup (when required)

A plain branch at `HEAD` is insufficient when the worktree is dirty. Create a snapshot commit or stash-like reference that includes:

- tracked modifications
- deletions
- untracked files

Acceptable approaches:

- temporary backup branch with a real snapshot commit
- `git stash push --include-untracked` followed by a named backup ref, if the workflow restores it safely
- an explicit patch plus untracked archive only when git refs are unavailable

Record the backup ref and tree:

```bash
git rev-parse <backup-ref>^{tree}
```

## Rewrite Branch Backup

For branch rewrite, a backup branch at the original tip is required:

```bash
git branch backup/<name>-<timestamp>
git rev-parse backup/<name>-<timestamp>^{tree}
```

## Final Verification

**With backup:**

```bash
git status --short
git diff --check <backup-ref> HEAD
git rev-parse <backup-ref>^{tree}
git rev-parse HEAD^{tree}
```

The final two tree hashes must match. If they do not, stop and report the mismatch.

**Without backup (same-session fast path):** clean `git status` after commits is sufficient.

## Safety Rules

- Do not push.
- Do not delete backups unless the user asks.
- Do not use `git reset --hard` except for an explicit user-approved rollback.
- Do not hide verification failures.

