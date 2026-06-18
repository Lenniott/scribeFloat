---
id: "0031"
title: Build /orient skill
status: active
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Build /orient skill

Auto-invoked at session start. Two-speed:
- No context given → reads CONTEXT.md index, asks what we're working on
- Context given → loads the relevant layer (engineering, design, product)

Lives in global skills (`~/.claude/skills/`) — cross-repo.

Also includes a session wrap signal: when conversation has been long and decisions have been captured, surfaces "we've covered a lot — here's what's written, consider starting a fresh session."
