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

---

## Cut (do not recreate without an ADR + human OK)

Deleted from the spine on purpose: `architecture.md`, `action-flows.md`, `components.md`, `engineering/`, `backlog/`, `explorations/`, `audits/`, `features/`, UI review essays. Prefer code + ADRs + `.scratch/` over bringing those trees back.

---

## Update rules

- Make an architectural decision → add a file under `adr/` and update `adr/README.md`
- Effort work / Known issues → `.scratch/<effort-slug>/` (see `agents/issue-tracker.md`)
- Do not invent replacement architecture essays to fill the cut
