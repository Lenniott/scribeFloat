---
status: captured
date: 2026-06-18
produces: CONTEXT.md, ADR-0001 through ADR-0005, docs/backlog/active/0014-0035
---

# Exploration: Domain modeling + documentation system design

**Date:** 2026-06-18  
**Status:** Captured → decisions written to CONTEXT.md, ADRs, and backlog

---

## What this conversation resolved

### Domain language
All resolved terms are now canonical in `CONTEXT.md`. Key shifts:
- **Note** replaces HistoryRecord — a living, editable entity, not an archival log entry
- **Source** is a content piece within a Note (transcript, written, web, etc.)
- **Scribe / Dictate** are capture profiles of the same technology, not distinct systems
- **App / Area / Home** replace Shell / Dashboard
- **Triage** is per-Note, universal — any agent action goes to Triage
- **Domain / Artifact** are the knowledge layer above Notes

### Documentation system design (not yet built)
The conversation surfaced a documentation architecture to build. Nothing below is implemented yet.

**Proposed structure:**
```
CLAUDE.md              ← short entry point only — what is this, where to go
CONTEXT.md             ← domain glossary (done)
docs/adr/              ← hard architectural decisions (5 written this session)
docs/backlog.md        ← prioritised stories linked to ADRs (updated this session)
docs/explorations/     ← this folder — lightweight capture of ideation sessions
docs/prd/              ← proposals and intent documents (design-brain-prd.md lives here)
docs/engineering/      ← layer rules, async rules, IPC patterns (extracted from CLAUDE.md)
docs/design/           ← UX rules, component decisions (or keep in skills)
```

**Skills to build:**
- `/orient` — auto-invoked at session start; reads CONTEXT.md index; progressive disclosure based on stated goal; two-speed (no context: ask what we're doing; context given: load the right layer)
- Session wrap signal — surfaces when a conversation has been long and significant decisions have been made; prompts to finish and start fresh

**UI Taxonomy:**
- Written and saved to iCloud Skills at `Skills/Taxonomy/ui-taxonomy.md`
- Reformatted with decision ladder front-loaded
- Pointed to from `.cursor/skills/ui-enforcement/references/README.md`

---

## What to do next (suggested order)

1. **Short-circuit CLAUDE.md** — extract engineering rules into `docs/engineering/`, shorten CLAUDE.md to an index
2. **Build `/orient` skill** — start with a simple version: read CONTEXT.md, ask one question
3. **Work the backlog** — start with the safest story: rename `audio_input_type` (zero behaviour change, low risk)
4. **Dictate always writes a Note** — enables Triage to work uniformly across all capture methods

---

## Open threads not resolved

- Panel → Region rename in code (CLAUDE.md open question)
- `/orient` skill design (where does it live — global skills or repo?)
- Whether the engineering docs extracted from CLAUDE.md live in `docs/` or as a skill
