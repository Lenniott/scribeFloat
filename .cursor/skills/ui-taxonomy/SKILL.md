---
name: ui-taxonomy
description: Classify UI work into the correct taxonomy level before building. Use when creating a new UI element, naming a component, deciding where something belongs in the system, or when another skill needs a shared vocabulary for UI structure.
---

# UI Taxonomy

Classify every UI element before you build it. Work down the decision ladder — stop at the first yes.

## Decision ladder

1. Is it just a named value? → **Token**
2. Will multiple components reuse it? → **Primitive**
3. Is it a single, indivisible user action? → **Component**
4. Is it one action that needs multiple components working together? → **Pattern**
5. Can the thing it's about be clearly named, and does it contain multiple patterns, components, or information about that thing? → **Section**
6. Is it a fixed structural area of the layout, regardless of content? → **Region**

## Level definitions

Consult when a rule fires and the level needs clarifying.

**Token** — a single design value. Not markup, not behaviour. Exists only to provide named values that other levels consume.

**Primitive** — styled HTML or a small building block reused by multiple components. Its reason for existing is reuse, not direct product-level use.

**Component** — a complete, understandable unit for a single user action. Contains enough information for the user to understand what it does and how to use it.

**Pattern** — a complex action. Combines multiple components into one interaction flow. Still one action from the user's perspective, but requires multiple parts working together.

**Section** — a contained mental model. Groups patterns, components, and information around one coherent object or subject. The boundary is the named thing it is about.

**Region** — a fixed structural area of the layout. The room, not the furniture. Present regardless of what content appears inside it.
