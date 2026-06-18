# Rewrite Branch Workflow

Use when the user already made messy commits and wants the branch repackaged.

## Procedure

1. Identify current branch and candidate base.
2. Ask the user to confirm the base if more than one is plausible.
3. Create an exact backup branch at the current branch tip.
4. Record the final tree of the messy branch.
5. Show the commit range to rewrite with `git log --oneline <base>..HEAD`.
6. Propose the new commit plan before mutation.
7. After approval, reset the branch to the base while preserving changes in the worktree.
8. Rebuild the branch as clean commits using the dirty-worktree staging process.
9. Verify the final tree equals the original messy branch tree.

## Base Selection

Prefer, in order:

- user-specified base
- upstream tracking branch merge-base
- obvious target branch such as `main`, `master`, `develop`, or release branch

If uncertain, ask. Do not guess a base for history rewrite.

## Safe Reset

Use only after approval:

```bash
git reset --mixed <base>
```

Never use `git reset --hard` for curation unless the user explicitly approves rollback.

## Output

Report:

- original branch
- base commit
- backup branch
- old commit range
- new commits
- final tree verification

