---
id: "0001"
title: Dissolve context/ folder — move contents to skills/ and docs/
status: done
exploration: 2026-06-18-tooling-and-doc-system-design.md
---

# Dissolve context/ folder

Move all contents of `context/` to their new homes:

| Current path | New path |
|---|---|
| `context/design-skill/` | `skills/design-skill/` |
| `context/code-practice-skill-builder/` | `skills/code-practice-skill-builder/` |
| `context/architecture.md` | `docs/architecture.md` |
| `context/action-flows.md` | `docs/action-flows.md` |
| `context/components.md` | `docs/components.md` |
| `context/README.md` | merge into `AGENTS.md` / `CONTEXT.md` |

After moving:
- Update all cross-references in `CLAUDE.md`, `AGENTS.md`, and any skill files that reference `context/` paths
- Delete `context/` directory
- Verify `skills/build.sh` (story 0002) copies the new skill paths correctly
