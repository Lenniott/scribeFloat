# Dirty Worktree Workflow

Use when the user has uncommitted changes and wants useful commits.

## Procedure

1. Confirm current branch, current `HEAD`, and dirty files.
2. Create an exact backup snapshot of the current worktree, including untracked files.
3. Inspect the diff and group changes by behavior, not by file type.
4. Present a commit plan before mutation:
   - commit title
   - files/hunks included
   - rationale
   - commit message
5. After user approval, stage one group at a time.
6. Commit each group.
7. Verify final tree equals the backup snapshot tree.

## Plan Output

Use this shape:

```md
Backup:
- branch/snapshot:
- base:

Commit 1: type(scope): subject
Files:
- path
Rationale:
- why these changes belong together
Message:
- subject + body
```

## Execution Rules

- Prefer `git add <specific files>`.
- Use hunk staging when one file mixes unrelated changes.
- Before each commit, inspect `git diff --cached --stat`.
- After each commit, run `git status --short`.
- If a file contains inseparable behavior plus formatting churn, say so in the plan.

