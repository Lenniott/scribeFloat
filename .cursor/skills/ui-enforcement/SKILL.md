---
name: ui-enforcement
description: >-
  Enforces ScribeFloat UI consistency — typography role classes, design tokens,
  and component primitives. Use when writing or editing Svelte/CSS frontend code,
  Tailwind classes, forms, labels, screens, onboarding, or when the user mentions
  design system, typography pass, sf-* classes, or UI consistency.
---

# UI enforcement

Gate for all frontend work in this repo. **Load the relevant chapter only** — do not read every reference up front.

## When to use

- Adding or editing `.svelte` screens or components
- Choosing Tailwind classes for text, color, spacing, or surfaces
- Typography consistency pass or migrating inline styles to role classes
- Reviewing whether a PR matches the design system

## Workflow

```
1. Apply the cheat sheet in .cursor/rules/ui-enforcement.mdc (auto-attached on frontend files)
2. Only if unsure → open the reference chapter for that aspect
3. Query live specs:  python3 context/design-skill/query.py ds get <path>
4. Extend primitives — do not copy class strings into screens
5. Before finishing:  npm run check:ds
```

Do **not** read this skill file on every frontend edit — the rule + preToolUse hook carry the cheat sheet. Open chapters only for non-obvious cases.

## Aspect index (progressive disclosure)

| Aspect | Status | Reference |
|--------|--------|-----------|
| **Typography** | Active — refactor in progress | [references/typography.md](references/typography.md) |
| **Color** | Active — audit complete, refactor pending | [references/color.md](references/color.md) |
| Spacing | Planned | [references/README.md](references/README.md) |
| Radius & shadows | Partial — `check:ds` rules exist | [references/README.md](references/README.md) |
| Surfaces & elevation | Planned | [references/README.md](references/README.md) |
| Motion | Planned | [references/README.md](references/README.md) |
| Component primitives | Planned | [references/README.md](references/README.md) |

Read **one chapter** for the aspect you are changing. Cross-read a second only when the task clearly spans aspects (e.g. a new form field = typography + spacing).

## Hard rules (all aspects)

- **Design skill first** for anything not covered in a reference chapter: `context/design-skill/SKILL.md`
- **`npm run check:ds`** must pass before committing frontend changes (when rules exist for that aspect)
- **No off-scale Tailwind defaults** in product UI (`text-base`, `text-sm`, etc.) — see typography chapter
- **Extend primitives** (`ConfigField`, `Button`, `TabPage`, …) — do not paste label/layout recipes into screens
- **Color at call site** for contextual states (destructive, active, muted); role classes own size/weight/tracking/case. Use **Option A** semantic fg: `text-fg`, `text-fg-dim`, `text-fg-muted` — not `text-fg/N`.

## Key paths

| Path | Purpose |
|------|---------|
| `src/app.css` | `sf-*` role classes (`@layer components`) |
| `docs/typography-audit.md` | Migration inventory (what still uses inline styles) |
| `scripts/check-design-system.mjs` | Automated token violations |
| `context/design-skill/query.py` | Live design system + UX playbook |

## Refactor status

Typography role classes are **defined** in `app.css`. Most screens still use **inline** tokens — migrating them is a separate pass. When editing a file, move it toward role classes; do not add new inline typography recipes.

## Agent enforcement (rules + hooks)

Optimized for **right-first-run** (fewer retry tokens):

| Layer | Path | When | Effect |
|-------|------|------|--------|
| **Cursor rule** | `.cursor/rules/ui-enforcement.mdc` | Frontend files in context | Self-contained cheat sheet |
| **preToolUse hook** | `.cursor/hooks/ui-enforcement-check.mjs` | Before `Write` / `StrReplace` | **Denies** if write chunk **or resulting full file** violates |
| **postToolUse hook** | same | After write | **`additional_context`** listing remaining violations in that file (primary feedback) |
| **afterFileEdit hook** | same | After `Write` / `StrReplace` | Full-file scan + `check:ds` (stderr log) |
| **Tests** | `.cursor/hooks/ui-enforcement.test.mjs` | `npm run test:ui-enforcement` | Smoke tests for deny + follow-up |

Touching a frontend file requires migrating **the whole file** — partial edits that leave `uppercase`, `text-fg/N`, or inline label recipes are denied or flagged in postToolUse.

Shared cheat sheet: `.cursor/hooks/ui-enforcement-lib.mjs` (`CHEAT_SHEET`).

### Token strategy

- Rule carries the decision table → agent gets typography rules when editing `.svelte` without opening skill + reference files.
- preToolUse blocks `text-base`, `text-sm`, inline label recipes **before** they land → avoids fix-up turns.
- Read `references/typography.md` only for edge cases or migration work.

### Tightening later (post-migration)

Add deny rules for raw `text-label-*` / `text-body-*` in `ui-enforcement-lib.mjs` and `scripts/check-design-system.mjs` once inline usage is gone.
