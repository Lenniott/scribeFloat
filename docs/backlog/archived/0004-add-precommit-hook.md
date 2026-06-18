---
id: "0004"
title: Add pre-commit git hook — run build.sh as safety net
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Add pre-commit git hook for build.sh

Git pre-commit hook that runs `skills/build.sh` before every commit, ensuring `.cursor/skills/` and `.claude/` are never out of sync with `skills/` at commit time.

## Implementation

Create `.git/hooks/pre-commit` (or use a hook manager like `husky` if the project already uses one):

```bash
#!/bin/sh
bash skills/build.sh || exit 1
git add .cursor/skills/ .claude/
```

The `git add` after the build re-stages any files that `build.sh` updated, so the commit includes the sync result.

## Acceptance

- Committing a change to `skills/` automatically syncs `.cursor/skills/` in the same commit
- A failing `build.sh` aborts the commit with a non-zero exit

## Dependency

Requires story 0002 (build.sh).
