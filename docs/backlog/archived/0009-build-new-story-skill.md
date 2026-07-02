---
id: "0009"
title: Build /new-story skill
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Build /new-story skill

A Claude Code skill invoked as `/new-story` that guides creation of a well-formed story file in `docs/backlog/active/`.

## Behaviour

1. Asks (or accepts as args): title, optional ADR reference, optional exploration reference
2. Determines next sequence number by scanning `docs/backlog/active/` and `docs/backlog/archived/`
3. Creates `docs/backlog/active/NNNN-slug.md` with correct frontmatter and a starter body
4. Confirms the created path

## Skill location

`skills/new-story/` → built to `.cursor/skills/new-story/` and `.claude/commands/new-story/` by `build.sh`

## Acceptance

- `/new-story "Add triage surface"` creates the next-numbered file without prompting
- Sequence numbers never collide even if archived stories exist with the same number
