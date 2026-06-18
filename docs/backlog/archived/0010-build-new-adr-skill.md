---
id: "0010"
title: Build /new-adr skill
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Build /new-adr skill

A Claude Code skill invoked as `/new-adr` that guides creation of a well-formed ADR in `docs/adr/`.

## Behaviour

1. Asks (or accepts as args): decision title, context, the decision itself, consequences
2. Determines next ADR number by scanning `docs/adr/`
3. Creates `docs/adr/NNNN-slug.md` using the standard ADR template
4. Optionally updates `docs/adr/README.md` index with the new entry

## Skill location

`skills/new-adr/` → built to both tool locations by `build.sh`

## ADR template

```markdown
# ADR-NNNN: Title

## Status
Accepted

## Context
...

## Decision
...

## Consequences
...
```

## Acceptance

- `/new-adr "Float results are stored per-note"` creates the next-numbered ADR file
- `docs/adr/README.md` is updated with a one-line summary
