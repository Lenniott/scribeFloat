# Backup and Verification

The backup must preserve the exact value the user wants curated.

## Dirty Worktree Backup

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

After curation:

```bash
git status --short
git diff --check <backup-ref> HEAD
git rev-parse <backup-ref>^{tree}
git rev-parse HEAD^{tree}
```

The final two tree hashes must match. If they do not, stop and report the mismatch.

## Safety Rules

- Do not push.
- Do not delete backups.
- Do not use `git reset --hard` except for an explicit user-approved rollback.
- Do not hide verification failures.

