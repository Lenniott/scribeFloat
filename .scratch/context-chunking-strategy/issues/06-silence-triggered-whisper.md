---
title: Silence-triggered Whisper (parked)
labels: [wayfinder:research, needs-info]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

Status: needs-info — not on the frontier

# 06 — Silence-triggered Whisper (parked)

**What to build:** Nothing in this effort. Research/prototype later: while recording, treat a long silence as “this PCM span is done,” run Whisper on it, append lines with absolute timestamps, so Stop only waits on the tail.

This does **not** change chunk schema, filters, or vectors. Chunking still runs only after Stop and final speaker stamp. Extra shorter lines from silence cuts pack into the same speaker-then-size windows.

**Blocked by:** None — parked until an accuracy check exists. Do not start while 01–05 are the frontier.

## Must not do (ADR locks)

- Do not treat each Whisper job as a chunk
- Do not replace the segment array at Stop without an index rebuild
- Do not splice/merge lines in the middle if chunks still use array indexes
- Do not index while capture is still open
- Do not stamp speaker on a wave before diarization is flushed unless you restamp at Stop

## Unresolved before this is ready-for-agent

- WER at silence-boundary sentences vs one full-buffer pass (prompt carryover? overlap? accept glue errors?)
- Dictate-only prototype vs Record as well (a parked Dictate note already frames this as Dictate-first)

- [ ] Accuracy policy decided with a measured prototype
- [ ] Then a new implementation ticket that only appends waves into the frozen-Note model from ADR 01
