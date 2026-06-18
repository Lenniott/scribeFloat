---
id: "0002"
title: Create skills/build.sh — copy + mtime sync logic
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Create skills/build.sh

Single build script that keeps all agent tool skill copies in sync.

## Responsibilities

1. Copy `skills/` → `.cursor/skills/` (Cursor's skill location)
2. Copy `skills/` → `.claude/commands/` or `.claude/` project-level location (Claude Code)
3. Sync `AGENTS.md` ↔ `CLAUDE.md` by mtime — whichever is newer wins, overwrite the other

## Requirements

- Idempotent: safe to run multiple times
- Fast: only copies if source is newer (use `rsync --update` or equivalent)
- Exits 0 on success, non-zero on any copy failure
- Prints a one-line summary of what was synced

## Acceptance

- Running `bash skills/build.sh` from repo root produces no errors on a clean repo
- Editing a skill file and re-running copies the change to both `.cursor/skills/` and `.claude/`
- Editing `AGENTS.md` and re-running overwrites `CLAUDE.md` with its content (and vice versa)
