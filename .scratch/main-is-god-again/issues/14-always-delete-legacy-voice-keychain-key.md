---
title: Always delete legacy voice Keychain key
labels: [wayfinder:task]
status: open
assignee:
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Does startup always remove the old voiceprint encryption key from Keychain — even when the `voiceprints/` folder is already gone — so the abandoned feature leaves no Keychain ghost?

**Done when:** Key delete is not gated only on “profiles dir removed”; safe if key already absent; matches “as if voiceprint never happened.”
