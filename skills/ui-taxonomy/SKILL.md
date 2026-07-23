---
name: ui-taxonomy
description: Classify UI work into the correct taxonomy level before building. Use when creating a new UI element, naming a component, deciding where something belongs in the system, or when another skill needs a shared vocabulary for UI structure.
---

# UI Taxonomy

Classify every UI element before you build it. Work down the decision ladder — stop at the first yes.

## Order of decisions

1. Is it just a value? → **Token**
2. Will multiple components use it? → **Primitive**
3. Is it a single, indivisible action? → **Component**
4. Is it one action that needs multiple components working together? → **Pattern**
5. Can the thing it is about be clearly named, and does it contain multiple patterns, components, or information about that thing? → **Section**
6. Is it a fixed structural area of the interface regardless of content? → **Region**

---

## Token

**Definition**

A token is a single design value. It is not markup, not behavior, and not a reusable UI unit. It is the lowest level of the system and exists only to provide named values that other layers use.

**Decision rule**

Ask: is this just a value?

If yes, it is a token.

**Folder** — design values live in `app.css` / Tailwind config. No numbered folder; tokens are not components.

**Examples**
- `--color-brand` — the brand accent colour
- `--color-canvas` — the page background
- `sf-label-sm` — a typography role class
- `bg-canvas`, `text-fg-dim` — Tailwind token utilities

---

## Primitive — `1_primitives/` · `@primitives`

**Definition**

A primitive is styled HTML or a small UI building block that is used by multiple components. A primitive can contain other primitives. Its reason for existing is reuse across components, not direct use as a meaningful product-level unit.

**Decision rule**

Ask: will this be used by multiple components?

If yes, make it a primitive.

If no, it is probably just part of a component rather than its own taxonomy item.

**Examples**
- `ScrollBody` — `overflow-y-auto` scroll container used inside NoteList, UploadQueue, SettingList
- `PanelHeader` — top chrome with title + action slots, used in capture, notes detail, settings
- `PanelFooter` — bottom chrome for primary actions; used in capture, detail pane
- `Modal` — focus-trap overlay; used by delete confirmation and any future dialog
- `Chip` — small badge/label; used in NavItem, FilterPanel
- `StatusDot` — pulsing recording-state dot; used in TitleBar and Dictate HUD
- `TextField` — labelled text input; used in mic name, speaker name fields

---

## Component — `2_components/` · `@components`

**Definition**

A component is a base action. It is a complete, understandable unit that lets a user initiate, record, or view something. It should contain the information needed for the user to understand what it does and how to use it.

**Decision rule**

Ask: is this a single, indivisible action?

If yes, it is a component.

**Examples**
- `Button` — one action (primary, destructive, ghost, normal, active)
- `Toggle` — one on/off decision
- `NoteCard` — one note row: user can select, copy, view, or delete it
- `UploadItem` — one queued file with progress, status, and actions
- `NavItem` — one navigation destination: icon + label + optional badge
- `Toast` — one transient notification
- `Waveform` — one live PCM visualizer

---

## Pattern — `3_patterns/` · `@patterns`

**Definition**

A pattern is a complex action. It combines multiple components into a single interaction flow or coordinated action. A pattern exists when one action needs multiple parts, steps, or controls to work together.

**Decision rule**

Ask: is this still one action, but it needs multiple components working together?

If yes, it is a pattern.

**Examples**
- `NoteComposer` — one action (create a note), but needs a TextField + Button working together
- `NoteList` — one action (browse notes), but needs multiple InlineNote components with shared scroll state
- `UploadQueue` — one action (manage the import queue), built from multiple UploadItem rows
- `Accordion` — one expand/collapse flow, coordinates multiple AccordionRow children

---

## Section — `4_sections/` · `@sections`

**Definition**

A section is a contained mental model. It groups multiple patterns, components, or information around the same thing. The important boundary is that it is about one coherent object, subject, or area of understanding from the user's perspective.

**Decision rule**

Ask two questions.

First: can the object or thing this is about be clearly named?

Second: does it contain multiple patterns, components, information, or related content about that same thing?

If both are yes, it is a section.

**Examples**
- `NoteDetailPane` — about one note: transcript, metadata, actions, all together
- `FilterPanel` — about the tag vocabulary: filter rows, active count, clear action
- `SettingsPanel` — about app settings: routes to the correct settings tab
- `WelcomeStep` / `ModelDownloadStep` — each about one named onboarding moment

---

## Views — `5_views/` · `@views`

Views are not a taxonomy level. They are route-level page compositions — they assemble regions, sections, patterns, and components into a full screen. Each view corresponds to one SvelteKit route or satellite window.

**Examples**
- `home.svelte` — Home screen (recent notes, stats)
- `notes.svelte` — Notes screen (list + detail)
- `capture.svelte` — Scribe capture overlay
- `dictate.svelte` — Floating dictate HUD (satellite window)
- `onboarding.svelte` — Onboarding flow (satellite window)

---

## Region — `6_regions/` · `@regions`

**Definition**

A region is a fixed structural area of the layout. It is part of the fundamental frame of the interface, regardless of what content appears inside it. A region is more like the room than the furniture.

**Decision rule**

Ask: is this an area of the interface that is regularly there and stays in the same place regardless of its contents?

If yes, it is a region.

**Examples**
- `AppSidebar` — always on the left; its route icons change state but it never moves
- `SettingsSidebar` — replaces AppSidebar in settings mode; structurally the same slot
- `TitleBar` — always at the top of the shell; content (recording state, new note button) varies but the bar is always there

---
