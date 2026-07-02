---
id: "0030"
title: Populate docs/engineering/ — extract rules from CLAUDE.md
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Populate docs/engineering/

Create `docs/engineering/` and extract verbatim from CLAUDE.md into focused files. Each file is loaded only when the task touches that domain — never upfront.

## Files to create

| File | Content extracted from CLAUDE.md |
|---|---|
| `layer-rules.md` | Layer diagram, ownership rules, how to add IPC command, how to add a feature |
| `async-rules.md` | Async rules, state machines |
| `platform-rules.md` | macOS main-thread rules, audio drain / MicSession rules, platform code rules |
| `debugging.md` | Bug investigation table, Whisper debugging section, unexplained constants rule |
| `config-rules.md` | Config changes section |

## Also create

- `docs/scribe-ui-review.md` — extract Scribe UI regression rules from CLAUDE.md (parallel to existing `docs/history-ui-review.md`)

## Pointer table for CLAUDE.md

After extraction, CLAUDE.md pointer table should read:

```
docs/architecture.md            ← C4 diagrams, call chain overview
docs/engineering/layer-rules.md ← Layer rules, ownership, IPC patterns, adding features
docs/engineering/async-rules.md ← Async, state machines
docs/engineering/platform-rules.md ← macOS threading, audio drain
docs/engineering/debugging.md   ← Bug table, Whisper debugging
docs/scribe-ui-review.md        ← Before touching Scribe UI
docs/history-ui-review.md       ← Before touching History UI
```

## Depends on

Story 0036 (architecture.md split — diagrams stay in architecture.md, layer rules move to engineering/).

## Acceptance

Each engineering doc is self-contained and loadable without the others. CLAUDE.md contains no content that duplicates these files.
