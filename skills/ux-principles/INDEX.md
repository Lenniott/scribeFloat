---
type: map
topic: navigation
when: You don't know which file to read, or you want a full picture of what this knowledge tree contains.
---

# UX Principles — Knowledge Map

> **When to read this**: You want a complete picture of the knowledge tree, or the task table in SKILL.md wasn't specific enough.

This file does not contain design rules — it maps which file contains what, and how to navigate between them.

---

## Entity map

Each entry below is one file. Read the "covers" column to decide if you need it.

### `user-context.md` — Context
Who ScribeFloat users are, what they need, how the product should feel, and what to avoid. Also contains the ScribeFloat Design DNA — the specific interpretations that distinguish ScribeFloat from generic UX advice.

Read this when: starting on a new screen; unsure whether a design decision fits the product; reviewing whether something "feels like ScribeFloat".

Cross-references: `principles.md` (the *why* behind the feel), `checklist.md` (the litmus test)

---

### `principles.md` — Rules
Ten core principles. These are the source of truth for design decisions. The rest of the files apply these principles to specific domains.

Read this when: justifying a decision; reviewing work; a rule in another file feels arbitrary and you want the reasoning.

Cross-references: All other files — every pattern traces back here.

---

### `interaction.md` — Patterns
Forms, inputs, buttons, toggles, menus, filters, and handling AI-generated or sensitive data. Includes the full "do / don't" quick-reference for interaction.

Read this when: building any interactive element — form, button, dropdown, toggle, filter, or AI data display.

Key principles: #2 (cognitive load), #5 (progressive disclosure), #6 (feedback), #8 (familiar patterns)

Cross-references: `principles.md`, `accessibility.md` (focus + ARIA), `visual-design.md` (spacing)

---

### `typography.md` — Patterns
The five-step type scale, weight rules, rendering, hierarchy, and formatting. Includes the typography "do / don't" quick-reference.

Read this when: choosing a text size or weight; setting up hierarchy for a new screen; reviewing typographic consistency.

Key principles: #1 (time), #2 (cognitive load), #7 (common case)

Cross-references: `visual-design.md` (spacing context), `checklist.md` (type checks)

---

### `motion.md` — Patterns
Animation timing by interaction type, scale rules, when to skip animation entirely, purpose taxonomy, and reduced-motion. Includes the motion "do / don't".

Read this when: adding any transition, animation, or hover effect; reviewing whether motion is justified.

Key principles: #1 (time), #4 (real context)

Cross-references: `visual-design.md` (performance), `accessibility.md` (reduced-motion)

---

### `accessibility.md` — Rules
Focus management, ARIA labels and roles, tooltip rules, colour contrast, and document-level concerns. Includes the accessibility "do / don't".

Read this when: implementing focus management, ARIA attributes, keyboard navigation, or contrast checks.

Key principles: #3 (honest and predictable), #6 (feedback)

Cross-references: `interaction.md` (forms + disabled states), `typography.md` (contrast), `motion.md` (reduced-motion)

---

### `visual-design.md` — Patterns
Surface elevation model, card types, spacing scale, semantic colour usage, border radius, scroll fade masks, responsive breakpoints, and rendering/performance rules. Includes layout and performance "do / don't".

Read this when: working with surfaces, depth, elevation, cards, spacing, colour, scroll overflow, or performance.

Key principles: #4 (real context), #7 (common case), #10 (absorb complexity)

Cross-references: `typography.md` (hierarchy), `interaction.md` (spacing around inputs), `motion.md` (compositing)

---

### `data-display.md` — Patterns
Number formatting, AI-generated content labelling, data clearance messaging, empty/loading/error states, and feedback placement.

Read this when: building any view with data, AI output, or state-dependent UI.

Key principles: #3 (honest), #6 (feedback), #9 (show the explanation)

Cross-references: `interaction.md` (feedback placement), `visual-design.md` (skeleton screens)

---

### `ux-laws.md` — Reference
Quick-reference cheat sheet: 20+ UX laws grouped by category (interaction, perception, memory, attention, decision-making, experience, complexity), each with a practical ScribeFloat takeaway.

Read this when: something feels harder to use than it should and you need to diagnose why; reviewing design decisions; writing a justification.

---

### `checklist.md` — Checklist
Pre-ship review gate covering hierarchy, states, accessibility, dark mode, performance, typography, design consistency, and AI/data handling. Ends with the ScribeFloat litmus test.

Read this when: reviewing any interface before merging or shipping.

Cross-references: All files — the checklist synthesises every chapter.

---

## Cross-file dependency graph

```
user-context ──────────────────────────────── principles
                                                  │
              ┌───────────────┬─────────────┬────┤────────────┐
              │               │             │                  │
         interaction     typography       motion         accessibility
              │               │             │                  │
              └───────────────┴─────────────┴──── visual-design
                                                       │
                                                  data-display
                                                       │
                                                   checklist ← ux-laws
```

Most tasks touch one or two nodes. Use the graph to decide if you need to follow a cross-reference.
