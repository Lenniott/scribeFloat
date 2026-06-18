---
id: "0036"
title: Split architecture.md — diagrams vs engineering rules
status: done
---

# Split architecture.md — diagrams vs engineering rules

`docs/architecture.md` currently mixes C4 diagrams (visual, rarely changes) with engineering rules (layer call chain, ownership, IPC patterns). Agents doing frontend work load the whole file to get the diagrams; agents doing backend work load it for the rules. Neither needs the other.

## Split

**Keep in `docs/architecture.md`:**
- C4 diagrams (Context, Container, Component levels in Mermaid)
- High-level data flow description

**Move to `docs/engineering/layer-rules.md`:**
- Layer call chain rules
- Hard ownership rules (HistoryService, OutputService, AudioService, PermissionsService)
- How to add an IPC command
- How to add a new feature
- Platform adapter convention

## Why this order

Architecture.md is the entry point for understanding the system. Layer-rules.md is the entry point for building inside it. They answer different questions and should be loaded at different times.

## Acceptance

An agent doing a UI task reads `architecture.md` for system context without loading backend engineering rules. An agent adding a Rust service reads `layer-rules.md` without loading C4 diagrams.
