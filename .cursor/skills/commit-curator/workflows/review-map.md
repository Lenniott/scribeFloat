# Review Map Workflow

Use when the user wants understanding, review guidance, or a developer handoff without changing git.

## Procedure

1. Inspect current branch, dirty state, and recent commits.
2. Determine the review range:
   - dirty worktree
   - commits since base
   - user-specified range
3. Group changes by behavior.
4. Produce a review map that explains intent, touched files, risks, and suggested commit structure.

## Output Shape

```md
Behavior: concise behavior name
Files:
- path
Intent:
- what changed from the user's/product point of view
Review notes:
- what a developer should scrutinize
Risk:
- likely failure mode or integration concern
Suggested commit:
- type(scope): subject
```

## Review Focus

Call out:

- prototype shortcuts
- broad formatting churn
- duplicated state paths
- swallowed errors
- fragile sequencing
- files that reveal ownership boundaries
- tests missing for the behavior

Do not mutate files or git state in this workflow.

