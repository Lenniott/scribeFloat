---
title: Sanitize transcript HTML
labels: [wayfinder:task]
status: open
assignee:
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

How do we stop transcript markdown from becoming unsafe HTML in the webview (`{@html}` / wide markdown options), without breaking normal transcript display?

**Done when:** User-influenced transcript text cannot inject HTML/handlers into the UI; approach recorded (sanitize, safer render, or both).
