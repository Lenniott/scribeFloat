# Approval Gate

**Turn 1 is plan-only. Turn 2 is execution-only.**

Agents skip this gate when the user says "commit" — that wording means *how* to commit, not *commit now without showing the plan*.

## Turn 1 — Plan (mandatory)

**Allowed:** read-only git — `git status`, `git diff`, `git log`, `git branch`, `git show`, `git rev-parse`

**Forbidden:** anything that mutates git state or the worktree for curation purposes:

- `git add`, `git commit`, `git stash`, `git reset`, `git checkout`, `git merge`, `git rebase`, `git cherry-pick`
- backup snapshot commits
- pushing

**Deliver:**

1. Backup *strategy* — snapshot ref name **or** "skipped (same-session commit)" when the fast path applies
2. Full commit plan using the shape in `workflows/dirty-worktree.md`
3. Explicit question: *"Proceed with these N commits as written, or change anything first?"*

**End Turn 1 with this line (verbatim intent, wording can vary slightly):**

```md
---
**Awaiting approval** — I have not run `git commit`. Reply **proceed**, **go**, or **lgtm** to execute this plan, or tell me what to change.
```

Do not run backup or commits in the same turn as the plan unless the user is **only** approving a plan you already presented in an earlier message in this thread.

## Turn 2 — Execute (only after approval)

User approval phrases (any of):

- proceed / go / lgtm / looks good / yes / do it / ship it / commit it

User revision phrases (back to Turn 1):

- change split / combine commits / different message / drop commit N / wait

**Then, in order:**

1. Create backup snapshot **only if** not using the same-session fast path (see `references/backup-and-verification.md`)
2. Stage and commit one group at a time per the approved plan
3. Verify: tree match against backup **if** backup was used; otherwise confirm clean worktree
4. Report commit SHAs and `git log --oneline` for the new commits

## Ambiguous requests

| User says | Agent does |
|-----------|------------|
| "commit these logically" / "split into commits" / "what's the best split?" | Turn 1 plan only |
| "commit" (after plan was shown, user replied proceed) | Turn 2 execute |
| "commit" (no plan in thread yet) | Turn 1 plan only |
| "just commit everything" / "one commit is fine" | Turn 1 plan with single commit; still wait for approval |
| Agent just built the changes; user says `/commit-curator` | Turn 1 plan; **backup skipped** on Turn 2 unless rewrite/mixed worktree |
| "review map" / "explain the commits" | `workflows/review-map.md` — no commits |

## Common failure mode

Combining plan + backup + commits in one response because:

- the user said "commit"
- the split seemed obvious
- executing felt faster

**Fix:** treat every `/commit-curator` invocation as Turn 1 unless this thread already contains an approved plan and the user's latest message is clearly approval.
