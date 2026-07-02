# Explorations

Lightweight captures of ideation and design sessions. Not decisions — decisions go to `docs/adr/`. Not stories — stories go to `docs/backlog/active/`.

## Folders

| Folder | When to use |
|--------|-------------|
| `active/` | Decisions not yet fully captured as ADRs or stories |
| `captured/` | All decisions have corresponding ADRs or stories — move here when done |
| `superseded/` | Replaced by a newer exploration in the same lineage — kept for its reasoning trail, not because it's neglected |
| `stale/` | Older than 30 days with no linked ADR or story — move here to flag for review |

**When an exploration's status changes, move the file to the matching folder.**

## Index

### active/
| File | Produces |
|------|----------|
| [2026-07-01-context-hydration-pipeline.md](active/2026-07-01-context-hydration-pipeline.md) | Chunk/block/pack context extraction + retrieval design — current |

### superseded/
| File | Superseded by |
|------|------|
| [design-brain-prd.md](superseded/design-brain-prd.md) | Flow 1 (Tags) unaffected; Flow 2 superseded by `2026-07-01-context-hydration-pipeline.md` |
| [knowledge-layer-intent.md](superseded/knowledge-layer-intent.md) | `2026-07-01-context-hydration-pipeline.md` (cross-note synthesis idea remains a later, undesigned phase) |
| [knowledge-orchestration.md](superseded/knowledge-orchestration.md) | `2026-07-01-context-hydration-pipeline.md` |
| [2026-07-01-context-extraction-engine-v1.md](superseded/2026-07-01-context-extraction-engine-v1.md) | `2026-07-01-context-hydration-pipeline.md` |

### captured/
| File | Date | Produces |
|------|------|----------|
| [2026-06-18-domain-modeling-and-doc-system.md](captured/2026-06-18-domain-modeling-and-doc-system.md) | 2026-06-18 | CONTEXT.md, ADR-0001–0005, stories 0014–0035 |
| [2026-06-18-tooling-and-doc-system-design.md](captured/2026-06-18-tooling-and-doc-system-design.md) | 2026-06-18 | stories 0001–0013 |
| [2026-06-19-notes-component-codemirror.md](captured/2026-06-19-notes-component-codemirror.md) | 2026-06-19 | ADR-0006–0009, stories 0044–0051 |

## Naming convention

`YYYY-MM-DD-short-slug.md` for session explorations. Undated files are intent/PRD docs that predate the convention.
