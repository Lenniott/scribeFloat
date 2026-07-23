---
name: ux-principles
description: >-
  ScribeFloat UX principles and design intent. Use when designing or reviewing
  any new screen, component, interaction pattern, or user flow — or when you
  need to understand WHY a design rule exists, not just what it is.
  Complements ui-enforcement (which governs tokens and class names).
---

# UX Principles

Design intent and interaction rules for ScribeFloat. These files tell you **why** things are built the way they are and **how** to make the right call when the token rules alone don't cover a decision.

> **Relationship to ui-enforcement**: ui-enforcement governs *how* (which tokens, classes, primitives). This skill governs *what and why* (interaction model, hierarchy, information design, trust).

---

## Quick navigate — go directly to what you need

| I'm doing… | Read |
|---|---|
| Starting a new screen or unfamiliar with ScribeFloat | `user-context.md` |
| Need to understand WHY a design rule exists | `principles.md` |
| Building a form, button, dropdown, or filter | `interaction.md` |
| Making typography decisions | `typography.md` |
| Adding or reviewing animation | `motion.md` |
| Accessibility review | `accessibility.md` |
| Working with surfaces, depth, spacing, or colour | `visual-design.md` |
| Empty / loading / error states, or data display | `data-display.md` |
| Quick UX law validation during review | `ux-laws.md` |
| Pre-ship checklist | `checklist.md` |
| Don't know where to start | `INDEX.md` |

---

## How to read this tree

Each file is self-contained. Files cross-reference each other at the top and bottom. You rarely need more than two files per task.

**Do not read every file** — that defeats the purpose. Start from the task table above, open one file, follow the `→ See also` links only if the task spans multiple areas.

---

## File index (brief)

| File | Type | Covers |
|---|---|---|
| `INDEX.md` | Map | Full entity map with one-line summaries |
| `user-context.md` | Context | Who SF users are, design intent, anti-patterns, DNA |
| `principles.md` | Rules | The 10 core design principles |
| `interaction.md` | Patterns | Forms, buttons, filters, AI data |
| `typography.md` | Patterns | Scale, hierarchy, rendering, weight |
| `motion.md` | Patterns | Timing, purpose, reduced motion |
| `accessibility.md` | Rules | Focus, ARIA, labels, contrast |
| `visual-design.md` | Patterns | Surfaces, cards, spacing, colour, radius, performance |
| `data-display.md` | Patterns | States, feedback, AI content, data formatting |
| `ux-laws.md` | Reference | Quick cheat sheet: 20+ UX laws |
| `checklist.md` | Checklist | Pre-ship review gate |
