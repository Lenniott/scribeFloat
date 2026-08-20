# Docs index

Thin keep-set. Load only what your session needs — see `AGENTS.md`.

---

## Keep

| Path | What's here | When to read |
|------|-------------|--------------|
| [../CONTEXT.md](../CONTEXT.md) | Domain glossary | Always — first |
| [../PRIVACY.md](../PRIVACY.md) | Privacy claims | Before network / data behaviour changes |
| [adr/](adr/) | Architecture Decision Records | Exploring; before architectural choices |
| [agents/](agents/) | Issue tracker, triage labels, domain-doc rules | Managing / wayfinding sessions |
| [assets/](assets/) | Static assets referenced by docs | As needed |

### Parked archive (not specs)

| Path | What's here | When to read |
|------|-------------|--------------|
| [ideas/](ideas/) | Closed-effort Known issues dumps and future destinations | Charting a new wayfinder map only |

**Do not implement from `docs/ideas/`.** Those notes are raw material for a future `/wayfinder` Destination, not a backlog and not Binding. If a file looks like a ticket (IPC names, capabilities, test plans), it is still parked until it has an open wayfinder issue.

---

## Cut (do not recreate without an ADR + human OK)

Deleted from the spine on purpose: `architecture.md`, `action-flows.md`, `components.md`, `engineering/`, `backlog/`, `explorations/`, `audits/`, `features/`, UI review essays. Prefer code + ADRs + `.scratch/` over bringing those trees back.

---

## Update rules

- Make an architectural decision → add a file under `adr/` and update `adr/README.md`
- Effort work / Known issues → `.scratch/<effort-slug>/` (see `agents/issue-tracker.md`)
- Closed-effort Known issues move wholesale into `ideas/` (see `agents/working-method.md`) — still not specs
- Do not invent replacement architecture essays to fill the cut
