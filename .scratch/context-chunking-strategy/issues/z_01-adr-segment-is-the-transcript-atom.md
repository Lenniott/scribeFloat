---
title: ADR — segment is the transcript atom
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-grok
blocked_by: []
parent: MAP.md
---

Status: closed (2026-08-18)

# 01 — ADR: segment is the transcript atom

**What to build:** A binding ADR so later tickets do not re-litigate this session. After it exists, a fresh agent should assume the transcript atom, the index shape, and the freeze-after-Stop rule — not the old “stored passage + line copy” sketch.

**Blocked by:** None — can start immediately.

## Resolution

[ADR-0015](../../../docs/adr/0015-whisper-line-is-the-transcript-atom.md) — Binding. Index row added. MAP Decisions so far point here.

- [x] Next ADR in `docs/adr/` written in the usual shape (Status Binding, Wayfinder provenance this effort), and listed in the ADR index
- [x] MAP “Decisions so far” points at the ADR; the old `embed_text` / `lines` candidate is marked superseded
- [x] The five locks above are in the ADR, not only in this ticket
