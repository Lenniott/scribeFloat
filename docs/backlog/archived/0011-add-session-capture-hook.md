---
id: "0011"
title: Add session hook — ask about uncaptured stories/ADRs, flag stale explorations
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Session capture hook

A Claude Code hook that fires once per session when either:
- 15 turns have passed with no writes to `docs/backlog/active/` or `docs/adr/`, OR
- Context usage passes 60%

## What it does

1. Asks: "Are there any stories or decisions from this session that need capturing?"
2. Suggests specific items it noticed (decisions made, patterns discussed, non-obvious choices)
3. Flags any exploration files whose status is not `captured` and that are older than 30 days, suggesting they be marked `stale`
4. Fires **once** per session — if dismissed, does not repeat

## Implementation options

- Claude Code `Stop` hook that checks turn count and context %
- Or a periodic hook on a turn interval

## Acceptance

- After 15 turns without a backlog write, the hook fires exactly once and asks the question
- It does not fire again in the same session after being dismissed
- It suggests stale explorations by filename

## Note

This is the most complex hook in the system. Design it last, after the simpler hooks (0003, 0004) are working.
