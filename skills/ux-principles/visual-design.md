---
type: patterns
topic: visual-design
when: Working with surfaces, depth, elevation, cards, spacing, colour, scroll overflow, responsive layout, or rendering performance.
see-also:
  - principles.md — #4 (real context), #7 (common case), #10 (absorb complexity)
  - typography.md — text hierarchy that lives on top of these surfaces
  - interaction.md — spacing around interactive elements
  - motion.md — compositing and animation performance
---

# Visual Design & Layout

> **When to read this**: You're making decisions about surfaces, depth, cards, spacing, colour tokens, scroll overflow, responsive layout, or rendering performance.

Visual design is how hierarchy becomes obvious at a glance. ScribeFloat needs a restrained, information-rich version of it — not sparse and decorative, but dense and purposeful.

→ **Principles at work**: #4 (real context), #7 (optimise for common case), #10 (absorb complexity)
→ **Typography on surfaces**: `typography.md`
→ **Animation compositing**: `motion.md`

---

## Surfaces & depth

Elevation is communicated by surface colour alone — not by shadow weight or border thickness. Only the top-level floating panel (PanelShell) uses a drop shadow.

**Dark mode**: lighter surface = higher elevation.
**Light mode**: darker surface = higher elevation.
Same token names; CSS vars swap automatically via the `data-theme` attribute on the root.

| Token | Tailwind class | Role | Example |
|---|---|---|---|
| `canvas` | `bg-canvas` | Page / window background only. Never a container. | App window background |
| `panel` | `bg-panel` | Main panel face, modal body. | Scribe panel, modal body |
| `card` | `bg-card` | Cards, note items, waveform bg, dividers. | Settings section bg, waveform container |
| `fill` | `bg-fill` | Hover bg, softer surface step. | Hovered rows, ghost button hover |
| `rim` | `bg-rim` | Borders, dividers, strong edge. | Input borders, toggle track border (off) |

---

## Cards

Keep the surface treatment consistent: same border radius, same padding, same edge definition within each card type.

**Data cards** — dense, tabular, small text, minimal padding. *Example: matter list items, document metadata.*

**Summary cards** — clear headline number, supporting context below. *Example: recording count, session duration.*

**Action cards** — clear CTA, brief description, minimal distraction. *Example: "Start new recording" prompt in an empty state.*

Do not over-card. If everything is in a card, nothing has hierarchy. Use cards for distinct, self-contained units of information.

---

## Spacing

Use a consistent spacing scale based on a 4px grid.

| Step | Value | Usage |
|---|---|---|
| 1 | 4px | Micro: icon to text gap |
| 2 | 8px | Tight: related items within a group |
| 3 | 12px | Standard: list items, compact rows |
| 4 | 16px | Comfortable: card content padding |
| 6 | 24px | Spacious: between sections |
| 8 | 32px | Major: page-level divisions |

---

## Colour usage

Use **semantic colour tokens**, not raw hex or OKLCH literals. Every token is defined in `src/app.css` (`--sf-*` / `@theme inline`) and switches automatically between dark and light mode.

| Token | Semantic job | Violation example |
|---|---|---|
| **brand** | Single primary CTA per screen | Using brand for waveform bars or decorative accents |
| **destructive** | Recording status dot and danger actions | Using destructive for non-critical warnings |
| **active** | Selection and on-states (tabs, toggle on) | Substituting active for the keyboard focus ring |
| **focus** | Keyboard focus rings only | Using focus as a general highlight colour |
| **warning** | Caution states | Using warning interchangeably with destructive |
| **success** | Confirmed/done states | Using success for waveform bars |

Build in grey surfaces first; add state colour only when it carries meaning. Pair colour with text or icon so meaning is never colour-only.

---

## Border radius

ScribeFloat uses three fixed radius values. Do not introduce others.

| Token | Value | Usage |
|---|---|---|
| `rounded-sm` | 2px | Chips, small tags, status dot containers, inner card elements |
| `rounded-md` | 4px | Buttons, inputs, cards, panels, modals — everything else |
| `rounded-full` | 9999px | Toggle knob and recording status dot only |

This is an instrument, not a consumer app. Unusual radius values introduce visual noise.

---

## Scroll overflow cues

In dense layouts, users need to know when content extends beyond the visible area. A gradient fade at the edge of a scrollable container signals "there is more" without taking up space or adding a permanent scrollbar.

- Fade masks must be `pointer-events: none` so they never block interaction with content underneath.
- Show or hide each mask based on scroll position: top mask appears when scrolled down, bottom mask appears when there is more content below.
- Match the gradient colour to the container's background using the same surface token, so the fade blends naturally in both light and dark mode.
- Use a `ResizeObserver` alongside scroll events to recalculate when the container or its content changes size.

---

## Responsive

ScribeFloat is laptop-first. Make sure layouts still work at narrower widths, in split-screen, and in resized laptop windows — but do not design around mobile as the main case.

| Breakpoint | Width | Context |
|---|---|---|
| Base | < 640px | Phone (edge case) |
| `sm` | 640px | Large phone / tablet |
| `md` | 768px | Tablet / laptop |
| `lg` | 1024px | Laptop (primary) |
| `xl` | 1280px | Desktop |
| `2xl` | 1536px | Large display |

---

## Rendering performance

Performance shapes trust as much as visuals do. A polished interface that feels slow is still poorly designed.

**Animate only `transform` and `opacity`** for 60fps compositing. Animating `width`, `height`, `top`, `left`, `margin`, or `padding` triggers layout reflow.

**Large `blur()` on `filter` or `backdrop-filter`** is expensive. Use sparingly.

**Never interleave DOM reads and writes in the same frame.** Batch all reads first, then writes.

**Use `will-change` only during active animations.** Add it on hover/focus, remove it when done. Pre-emptively adding it everywhere can *hurt* performance.

**Pause off-screen animations** with `IntersectionObserver`.

**Reserve space for scrollbars** with `scrollbar-gutter: stable` on any container that may or may not scroll. Without it, the layout shifts horizontally when a scrollbar appears or disappears.

**Use structural skeleton screens** that match the layout they'll replace, not generic spinners. → See `data-display.md`.

**Svelte-specific**: For real-time values that update on every frame (scroll position, waveform data), use a plain variable instead of a `$state` rune. Commit to the DOM directly via canvas API or direct element mutations. Use `$derived` for computed values — avoid recalculating inside `$effect` when `$derived` would do.

---

## What to avoid

- Marketing-site styling in working tools: oversized hero treatments, decorative gradients, large areas of empty space.
- Visual novelty that competes with the task: multiple accent colours, decorative iconography, playful shapes, rainbow status systems.
- Card overload: if every object gets the same container, padding, and weight, nothing feels important.

---

## Quick reference: do / don't

| Do | Don't |
|---|---|
| Use semantic colour tokens | Hardcode hex values |
| Use a consistent spacing scale (4px grid) | Eyeball padding and margin |
| Use **brand** for the single primary CTA per view | Introduce extra brand hues or decorative accents |
| Handle loading, empty, and error states | Only design the happy path |
| Test in both light and dark mode | Skip dark mode testing |
| Use scroll fade masks on clipped containers | Leave users guessing whether content continues |
| Use structural skeletons for loading | Show generic spinners |
| Use `scrollbar-gutter: stable` on scrollable containers | Let scrollbar appearance shift the layout |
| Animate only `transform` and `opacity` | Animate layout properties that trigger reflow |
| Pause off-screen animations | Let off-screen animations run indefinitely |
