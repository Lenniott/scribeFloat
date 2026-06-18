# Docs index

Human-readable documentation. Load only what your session needs — see `CLAUDE.md` for the session-type reading lists.

---

## Folders

| Folder / File | What's here | When to read |
|---|---|---|
| [../CONTEXT.md](../CONTEXT.md) | Domain model, app overview, reading order | Always — first |
| [architecture.md](architecture.md) | C4 diagrams, component maps, module map | Exploring the system; frontend work |
| [action-flows.md](action-flows.md) | Step-by-step user flows (source of truth) | Before changing any user-facing flow |
| [components.md](components.md) | UI component catalogue | Frontend building sessions |
| [engineering/](engineering/) | Focused rules for building: layers, async, platform, debugging, config | Building sessions — load the specific file, not the whole folder |
| [scribe-ui-review.md](scribe-ui-review.md) | Scribe screen regression rules | Before touching Scribe screens or navigation |
| [history-ui-review.md](history-ui-review.md) | History screen regression rules | Before touching History screens or components |
| [backlog/](backlog/) | Active stories (one file each) | Managing sessions; checking scope |
| [adr/](adr/) | Architecture Decision Records | Exploring sessions; before making architectural choices |
| [explorations/](explorations/) | Pre-decision explorations and intent docs | Exploring sessions |
| [features/](features/) | Per-feature implementation notes | Building a specific feature |
| [audits/](audits/) | Typography and colour audit inventories | UI enforcement sessions |

---

## Update rules

- Change a user-facing flow → update `action-flows.md`
- Change a UI component → update `components.md`
- Change layer ownership, call chain, or platform rules → update the relevant file in `engineering/`
- Make an architectural decision → write an ADR (`/new-adr`)
- Identify new work → write a story (`/new-story`)
