# Explorations

Lightweight captures of ideation and design sessions. Not decisions — decisions go to `docs/adr/`. Not stories — stories go to `docs/backlog/active/`.

## Folders

| Folder | When to use |
|--------|-------------|
| `active/` | Decisions not yet fully captured as ADRs or stories |
| `captured/` | All decisions have corresponding ADRs or stories — move here when done |
| `stale/` | Older than 30 days with no linked ADR or story — move here to flag for review |

**When an exploration's status changes, move the file to the matching folder.**

## Index

### active/
| File | Produces |
|------|----------|
| [design-brain-prd.md](active/design-brain-prd.md) | Float enrichment engine proposal |
| [knowledge-layer-intent.md](active/knowledge-layer-intent.md) | Knowledge layer intent doc |
| [2026-06-23-voiceprint-engine.md](active/2026-06-23-voiceprint-engine.md) | ADR-0011, stories 0052–0058 |

### captured/
| File | Date | Produces |
|------|------|----------|
| [2026-06-18-domain-modeling-and-doc-system.md](captured/2026-06-18-domain-modeling-and-doc-system.md) | 2026-06-18 | CONTEXT.md, ADR-0001–0005, stories 0014–0035 |
| [2026-06-18-tooling-and-doc-system-design.md](captured/2026-06-18-tooling-and-doc-system-design.md) | 2026-06-18 | stories 0001–0013 |
| [2026-06-19-notes-component-codemirror.md](captured/2026-06-19-notes-component-codemirror.md) | 2026-06-19 | ADR-0006–0009, stories 0044–0051 |

## Naming convention

`YYYY-MM-DD-short-slug.md` for session explorations. Undated files are intent/PRD docs that predate the convention.
