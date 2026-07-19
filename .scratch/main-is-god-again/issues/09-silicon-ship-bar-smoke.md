---
title: Silicon ship-bar smoke
labels: [wayfinder:task]
status: open
assignee:
blocked_by:
  - "07-remove-float-coming-soon-ui.md"
  - "08-delete-dead-multi-model-paths.md"
  - "12-bundle-only-models-no-runtime-fetch.md"
  - "13-sanitize-transcript-html.md"
  - "14-always-delete-legacy-voice-keychain-key.md"
  - "15-verify-all-bundled-models-before-load.md"
  - "16-least-privilege-ipc-per-window.md"
  - "18-mark-and-amend-adrs-for-reality.md"
parent: MAP.md
---

Ticket **17** was parked to Known issues (2026-07-19) and is **not** a smoke blocker.

## Question

On Apple Silicon, does a cold-ish run clear the ship bar: first-run/permissions → Dictate once → Record once → note in Notes with transcript → speaker rename cascades for that speaker → relaunch still shows the note?

**Done when:** Pass/fail recorded; failures either fixed as merge-blockers or explicitly parked in Known issues with human OK.
