---
id: "0012"
title: Add status frontmatter to all existing exploration files
status: active
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Add status frontmatter to explorations

Each exploration file should have `status: active | captured | stale` in its frontmatter so the session hook (story 0011) can identify stale ones.

## Files to update

Scan `docs/explorations/` and add/update frontmatter on each file.

## Status rules

- `active` — decisions not yet captured into ADRs or stories
- `captured` — all decisions have corresponding ADRs or stories (set this on `2026-06-18-tooling-and-doc-system-design.md` once stories 0001–0012 exist)
- `stale` — older than 30 days with no linked ADR or story

## Acceptance

- Every file in `docs/explorations/` has a `status:` field
- `2026-06-18-tooling-and-doc-system-design.md` is marked `status: captured`
