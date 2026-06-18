---
id: "0006"
title: Rename CLAUDE.md → AGENTS.md, create synced CLAUDE.md
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Rename CLAUDE.md → AGENTS.md

`AGENTS.md` is the universal agent instruction file (Cursor, Codex, any future tool). `CLAUDE.md` is Claude Code's file. They stay in sync via `build.sh` mtime comparison — whichever is newer wins.

## Steps

1. Rename `CLAUDE.md` → `AGENTS.md`
2. Create `CLAUDE.md` as a symlink to `AGENTS.md`, OR have `build.sh` copy the newer file to the other on every run
3. Update `build.sh` (story 0002) to handle the mtime sync
4. Verify Claude Code still picks up `CLAUDE.md` correctly after the change

## Decision

The exploration chose **copy on mtime** (not symlink) so both files are real files that any tool can read without symlink support.

## Acceptance

- Editing `AGENTS.md` and running `build.sh` overwrites `CLAUDE.md` with the same content
- Editing `CLAUDE.md` and running `build.sh` overwrites `AGENTS.md` with the same content
- Claude Code sessions load the correct content
