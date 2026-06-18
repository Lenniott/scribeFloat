---
type: patterns
topic: typography
when: Choosing a text size or weight; setting up hierarchy for a new screen; reviewing typographic consistency.
see-also:
  - principles.md — #1 (time), #2 (cognitive load), #7 (common case)
  - visual-design.md — spacing context, surface backgrounds that affect readability
  - checklist.md — typography pre-ship checks
---

# Typography

> **When to read this**: You're choosing a text size, weight, or style — or you're setting up the visual hierarchy for a new screen.

Typography is not decoration. It determines whether information feels legible, structured, and credible before anyone reads a full sentence. In dense professional tools, typography is the first filter for what matters. Weak hierarchy makes timestamps, speaker labels, audio duration, and file paths disappear into the page.

→ **Principles at work**: #1 (respect time), #2 (reduce cognitive load), #7 (common case dominance)
→ **Token values for each scale step**: `src/app.css` (`sf-*` role classes)
→ **Colour contrast rules**: `accessibility.md`

---

## Rendering

Apply `-webkit-font-smoothing: antialiased` for crisp rendering on high pixel-density screens.

Apply `text-rendering: optimizeLegibility` for better kerning and ligatures.

Subset fonts based on the content, relevant alphabet, or languages used. Do not ship 200KB of glyphs you will never render.

---

## Weight & sizing

**Never use font weights below 400.** Weights from 100–300 are unreadable on most screens.

**Medium headings look best at 500–600 weight.** Reserve 700 (bold) for emphasis or primary headings only.

**Font weight must not change on hover or selection.** Changing weight causes layout shift because the text physically moves. Use `color`, `background`, or `text-decoration` instead.

---

## Hierarchy

ScribeFloat uses five named scale steps. Use the semantic class (or `sf-*` helper) — never compose the parts individually.

Hierarchy comes from size, case, and opacity only. No bold (`font-weight` 600+) on body content. No custom tracking outside the three defined values (`tracking-normal`, `tracking-tight`, `tracking-stamped`).

| Scale | Tailwind class | Role | Usage |
|---|---|---|---|
| `display-lg` | `text-display-lg font-mono font-normal` | Large numeric readout. One per screen. | Recording timer only |
| `headline-sm` | `sf-headline-sm` (= `text-headline-sm font-mono uppercase tracking-stamped`) | Screen-level heading. Mono uppercase. | Modal titles, screen headers — one per view |
| `body-md` | `text-body-md font-sans font-normal` | Default reading text. | Descriptions, onboarding prose, transcript body |
| `label-md` | `text-label-md font-sans font-medium` | Control labels. | Form field labels, button text, accordion titles |
| `label-sm` / `label-sm-mono` | `text-label-sm … uppercase tracking-stamped` | Metadata captions. Sans or mono. | Section dividers, status chips, file sizes, hotkey chips |

**One `headline-sm` per view.** Two screen-level headings in the same view compete and flatten hierarchy.

---

## Formatting tips

Use `text-balance` on headings to prevent awkward line breaks.

Use `text-pretty` on body paragraphs for better widows/orphans handling.

Use `tabular-nums` for any numerical data so digits align in columns. This matters for timestamps, durations, and file sizes.

Use `truncate` or `line-clamp` in dense layouts to prevent text overflow.

Format long numbers and IDs in readable chunks: `4242 4242 4242 4242`, not `4242424242424242`.

---

## Quick reference: do / don't

| Do | Don't |
|---|---|
| Use font weight 400+ only | Use thin/light weights 100–300 |
| Keep heading weight at 500–600 | Change font weight on hover (causes layout shift) |
| Apply `antialiased` and `optimizeLegibility` | Ship unsubsetted fonts with unused glyphs |
| Use `tabular-nums` for numbers | Display numbers in proportional-width fonts |
| Use semantic text hierarchy via `sf-*` role classes | Style every text element individually with inline tokens |
| One `headline-sm` per view | Use multiple screen-level headings at the same weight |
