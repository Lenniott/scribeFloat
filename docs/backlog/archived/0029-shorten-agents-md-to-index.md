---
id: "0029"
title: Slim CLAUDE.md and AGENTS.md to pointer indexes
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Slim CLAUDE.md and AGENTS.md to pointer indexes

Both files are currently loaded into every agent context on every task. They contain ~250 lines of dense engineering rules — macOS threading, audio drain mechanics, History UI regressions, Whisper debugging — most of which are irrelevant to any given task.

Goal: reduce both files to ~60–80 lines of invariants + a pointer table. Agents pull specialist docs only when the task needs them.

## What stays in CLAUDE.md / AGENTS.md

- Skills pointer (write to `skills/`, run `build.sh`)
- Session capture rules
- Layer diagram (the 5-line call chain)
- Hard ownership rules (one line each: HistoryService, OutputService, AudioService, PermissionsService)
- Build & test commands
- Pre-commit checklist
- Out-of-scope list
- Pointer table: "if you're touching X, read Y"

## What moves out

All detailed rule sections move to `docs/engineering/` (see story 0030) and `docs/scribe-ui-review.md`. The pointer table in CLAUDE.md tells agents where to look.

## Depends on

Story 0030 (docs/engineering/ populated). Story 0036 (architecture.md split).

## Acceptance

CLAUDE.md fits on one screen. No section duplicates content that lives in a specialist doc.
