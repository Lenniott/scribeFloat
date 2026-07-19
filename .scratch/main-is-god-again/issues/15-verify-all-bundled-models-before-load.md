---
title: Verify all bundled models before load
labels: [wayfinder:task]
status: open
assignee:
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Do Whisper, VAD, and Sortformer all get the same integrity check before use after they are copied into the writable models folder (hash and/or re-seed from the signed app bundle on mismatch)?

**Done when:** Sortformer is not “filename only”; bad/missing copies fail clearly offline (no network redownload).
