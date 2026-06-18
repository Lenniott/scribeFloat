# Backlog

One file per story. Point an agent directly at a story file — no parsing needed.
Once a backlog item is done it must be moved to archived.

## Structure

```
docs/backlog/
  README.md          ← this file
  active/            ← open stories
    0001-slug.md
  archived/          ← completed or cancelled stories
    0001-slug.md
```

## Naming convention

`NNNN-kebab-slug.md` — four-digit zero-padded sequence number + descriptive slug.

## Story frontmatter

```yaml
---
id: 0001
title: Short human title
status: active | in-progress | done | cancelled
adr: ADR-NNNN          # optional — decision that generated this story
exploration: filename  # optional — exploration doc that generated this story
---
```

## Linking

- Stories reference ADRs by number: `ADR-0001`
- ADRs live in `docs/adr/`
- Explorations that produce stories update their frontmatter `status: captured`
