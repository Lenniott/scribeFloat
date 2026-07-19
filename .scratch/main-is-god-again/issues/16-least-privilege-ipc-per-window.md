---
title: Least-privilege IPC per window
labels: [wayfinder:task]
status: open
assignee:
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Can Dictate / onboarding / main shell each call only the Tauri commands they need — instead of one flat “every window may call everything” capability list?

**Done when:** Satellite windows cannot invoke unrelated high-impact commands; capability split is documented enough for the next agent.
