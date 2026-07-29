---
title: "Triage: Focus ring hidden or overridden by styling"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

`<select>` controls styled with `.sf-select` (used for Settings dropdowns) show no keyboard focus indicator at all: the class removes the default outline but, unlike the near-identical `.sf-input` right above it in `app.css`, never re-adds the focus-visible ring. Every other interactive primitive in the app already has a consistent focus ring via `.sf-focus-ring`/`.sf-input`.

## Question

Read the "Focus ring hidden or overridden by styling" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now** — trivial, one CSS rule added to `.sf-select` (`app.css:304-310`) plus a visual smoke check. Any reason not to just fix it?

## Findings

- Global baseline exists and is generally consistent: `src/app.css:335-341` (`@layer base`) sets `:focus { outline: none }` and `:focus-visible { outline: none; ring-2 ring-focus ring-offset-2 ring-offset-canvas }`, using the `--sf-focus` design token (orange, `oklch(0.73 0.17 55)` etc., defined per-theme at `src/app.css:120,157,193,234`). This is the "thick orange focus border" the ticket refers to.
- Most interactive primitives opt into a matching ring via the `.sf-focus-ring` utility (`src/app.css:325-328`: `outline-none focus-visible:ring-2 focus-visible:ring-focus ...`), applied in `Button.svelte:35`, `IconButton.svelte:35`, `Toggle.svelte:43,72`, `OptionGroup.svelte:33`, `NavItem.svelte:29`. Text inputs use `.sf-input` (`src/app.css:303`) which independently repeats the same `focus-visible:ring-2 ring-focus` styling. These are consistent with each other.
- Concrete bug found: `.sf-select` (`src/app.css:304-310`) sets `outline-none` but never re-adds a `focus-visible:ring-*` (unlike `.sf-input` right above it at line 303, which does both). Because this class lives in Tailwind's `@layer components`, it wins the cascade over the `@layer base` `:focus-visible` ring rule (base < components < utilities in Tailwind's layer order), so any `<select>` styled with `.sf-select` shows **no** focus indicator at all — outline removed, no ring added. Only known usage: `src/lib/ui/1_primitives/form/FieldRow.svelte:49` (`const selectClass = 'sf-select';`), used for select-type rows in Settings — this matches the ticket's "Settings/input controls" observation directly.
- Second, narrower override (likely intentional, not a bug): `src/lib/ui/3_patterns/NoteComposer.svelte:58,70` strips `outline-0 ring-0 ring-offset-0` from the `<textarea>` and the send `IconButton`, relying instead on the wrapping `<div>`'s `focus-within:ring-2 ...` (line 49) to show one ring around the whole composer. This still renders a visible ring on focus, just on the container rather than the control — worth confirming with design intent but not the "hidden" bug.
- CodeMirror editor (`src/app.css:342-349`, plus inline in `MarkdownEditor.svelte:76`) explicitly suppresses the focus ring on the writing pane ("caret only") — documented as deliberate, not a candidate fix.
- What a fix would touch: one CSS rule, `src/app.css:304-310` — add `focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-card` to `.sf-select` (mirroring `.sf-input`). Single file, single class, no component changes needed.
- Size estimate: trivial — a one-line CSS addition plus a visual smoke check of Settings dropdowns. Under 15 minutes of work; the ticket's broader "audit all controls" scope is unnecessary since the rest of the codebase already uses `.sf-focus-ring`/`.sf-input` consistently.
