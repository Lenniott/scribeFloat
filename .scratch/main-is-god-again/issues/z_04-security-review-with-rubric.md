---
title: Security review with rubric
labels: [wayfinder:research]
status: closed
assignee: research-agent
blocked_by: []
parent: MAP.md
---

## Question

Against a written rubric for this Tauri desktop app (IPC surface, filesystem/note storage, secrets/keychain, model files on disk, paste/accessibility, legacy voice data purge, dependency/risk hotspots), what security findings exist on the current spine?

Each finding needs: evidence, severity guess, and a suggested bucket (**merge-blocker** vs **Known issues**). The human sorts finally in a later ticket — do not close the book with “LGTM.”

## Resolution

Rubric + evidence-backed findings written to [`research/security-review.md`](../research/security-review.md).

Top suggested merge-blockers for human sort: (1) startup VAD download vs `PRIVACY.md` no-auto-network claim, (2) unsanitized markdown→`{@html}` transcript HTML under flat IPC, (3) legacy biometric vectors lingering in `history.jsonl` until compaction succeeds. Remaining items parked as suggested **Known issues** in the same file. No code fixes applied.
