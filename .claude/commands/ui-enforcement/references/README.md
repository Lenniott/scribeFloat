# UI enforcement — reference chapters

Progressive disclosure index. Each file is one aspect agents read **only when that aspect is in scope**.

| Chapter | File | Status |
|---------|------|--------|
| Typography | [typography.md](typography.md) | **Active** |
| Color | [color.md](color.md) | **Active** |
| Layout & scroll | [layout-scroll.md](layout-scroll.md) | **Active** |
| Spacing | `spacing.md` | Planned |
| Radius & shadows | `radius-shadows.md` | Planned (partial coverage in `check:ds` today) |
| Surfaces & elevation | `surfaces.md` | Planned |
| Motion | `motion.md` | Planned |
| Component primitives | `components.md` | Planned |

When adding a new chapter:

1. Create `references/<aspect>.md` with rules, role classes, banned patterns, and migration notes.
2. Add a row to the table above and to the index in `SKILL.md`.
3. Add matching rules to `scripts/check-design-system.mjs` when enforceable.
4. Link from `docs/README.md` if agents need a human-readable audit doc.
