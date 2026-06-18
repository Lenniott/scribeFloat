---
id: "0003"
title: Add PostToolUse hook — run build.sh on writes to skills/
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Add PostToolUse hook for skills/

Claude Code PostToolUse hook that auto-runs `skills/build.sh` whenever a file inside `skills/` is written.

## Hook spec

- **Event**: `PostToolUse`
- **Matcher**: tool is `Write` or `Edit` and the file path contains `/skills/`
- **Action**: `bash skills/build.sh`
- **On failure**: log to stderr, do not block the tool use

## Where to configure

`settings.json` in the project `.claude/` directory (or user-level settings if this should be global).

## Acceptance

- Writing any file under `skills/` from Claude Code causes `build.sh` to run automatically
- The hook does not fire for writes outside `skills/`
- If `build.sh` fails, Claude Code surfaces the error but does not retry in a loop

## Dependency

Requires story 0002 (build.sh) to exist first.
