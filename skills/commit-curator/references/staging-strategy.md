# Staging Strategy

Stage by intent.

## Prefer File-Level Staging

Use when each file belongs cleanly to one behavior:

```bash
git add path/a path/b
git diff --cached --stat
```

## Use Hunk Staging

Use when one file mixes multiple concerns:

```bash
git add -p path
```

For difficult splits, inspect with:

```bash
git diff -- path
git diff --cached -- path
```

## Avoid Broad Staging

Avoid `git add -A` until the remaining diff has been inspected and belongs to one final commit.

If using `git add -A`, state why the residue is safe to stage together.

## Formatting Churn

If formatting touches many unrelated files:

- isolate it in a separate commit when possible
- mention when behavior commits include unavoidable adjacent formatting
- do not mix formatter output with logic unless the file cannot be split safely

