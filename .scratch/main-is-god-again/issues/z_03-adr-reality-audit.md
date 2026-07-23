---
title: ADR reality audit
labels: [wayfinder:research]
status: closed
assignee: research-agent
blocked_by: []
parent: MAP.md
---

## Question

For each ADR in `docs/adr/`, is it **binding** (code matches), **aspirational** (decided, not built), or **superseded** (e.g. ADR-0011 by ADR-0014)?

Produce a findings file that an agent and a non-hardened maintainer can trust — evidence paths, not vibes. Recommend keep / mark / relocate per ADR. Do not delete history without an explicit recommendation.

## Resolution

8 binding (0001, 0003, 0006, 0008, 0009, 0012, 0013, 0014), 5 aspirational (0002, 0004, 0005, 0007, 0010), 1 superseded (0011). Highest-value marks: Sources model, ADR-0007 folders, and HistoryKind→quick/origin — glossary already ahead of code. Findings: [adr-reality-audit.md](../research/adr-reality-audit.md).
