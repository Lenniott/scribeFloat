---
id: "0005"
title: Write Cursor hook setup instructions doc
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Cursor hook setup instructions

Cursor does not have a built-in PostToolUse hook like Claude Code. Write a doc that tells Cursor (as an agent) how to self-configure so `build.sh` runs when skills are edited.

## What to produce

`docs/cursor-hook-setup.md` — a short doc that:

1. Explains that Cursor should add a `.cursorrules` or Cursor background-agent instruction to run `bash skills/build.sh` after editing any file under `skills/`
2. Gives the exact instruction text to paste into Cursor's settings
3. Notes that the pre-commit hook (story 0004) is the safety net if Cursor's hook is misconfigured

## Acceptance

A Cursor agent reading `docs/cursor-hook-setup.md` can follow the instructions to configure itself without further clarification.
