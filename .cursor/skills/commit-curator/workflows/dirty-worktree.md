# Dirty Worktree Workflow

Use when the user has uncommitted changes and wants useful commits.

**Read `references/approval-gate.md` first.** Do not skip Turn 1.

## Turn 1 — Plan only

1. Confirm current branch, current `HEAD`, and dirty files (read-only git).
2. Inspect the diff and group changes by **behavior**, not by file type.
3. Present a commit plan **before any mutation**:
   - commit title
   - files/hunks included
   - rationale
   - full commit message (subject + body)
4. State the backup strategy you will use on approval (ref name, snapshot method).
5. End with the **Awaiting approval** block from `references/approval-gate.md`.
6. **Stop.** Do not create backup refs or commits in this turn.

## Turn 2 — Execute (after user approval)

1. Create an exact backup snapshot of the current worktree, including untracked files.
2. Stage one approved group at a time.
3. Commit each group with the **approved** messages (do not rewrite messages without asking).
4. Verify final tree equals the backup snapshot tree.

## Plan Output Shape

```md
Backup (on approval):
- branch/snapshot: backup/<name>-<timestamp>
- base: <current HEAD sha>

Commit 1: type(scope): subject
Files:
- path
Rationale:
- why these changes belong together
Message:
subject line

body when needed

Commit 2: ...
```

## Execution Rules

- Prefer `git add <specific files>`.
- Use hunk staging when one file mixes unrelated changes.
- Before each commit, inspect `git diff --cached --stat`.
- After each commit, run `git status --short`.
- If a file contains inseparable behavior plus formatting churn, say so in the plan.
- If the approved plan must change mid-execution, stop and ask before continuing.
