---
id: "0008"
title: Migrate existing docs/backlog.md stories to individual files
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Migrate existing backlog stories

`docs/backlog.md` is a flat file with ~20 grouped stories. Migrate each to its own file in `docs/backlog/active/`.

## Source

`docs/backlog.md` — stories grouped by theme (Domain language alignment, Note data model, Capture profiles, Triage, App shell, UI taxonomy, Documentation system, Pre-existing).

## Process

1. For each story in `docs/backlog.md`, create `docs/backlog/active/NNNN-slug.md` with appropriate frontmatter
2. Preserve ADR references in frontmatter (`adr:` field)
3. Number sequentially starting from where the tooling stories (0001–0012) leave off
4. After migration, delete `docs/backlog.md`
5. Update `CLAUDE.md` / `AGENTS.md` to point to `docs/backlog/` instead of `docs/backlog.md`

## Note

The tooling stories (this file and siblings 0001–0012) use the first 12 slots. Start migrated stories from 0013.
