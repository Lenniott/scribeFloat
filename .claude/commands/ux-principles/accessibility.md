---
type: rules
topic: accessibility
when: Implementing focus management, ARIA attributes, keyboard navigation, labels, or colour contrast.
see-also:
  - principles.md — #3 (honest and predictable), #6 (feedback)
  - interaction.md — disabled states, tooltip usage in forms
  - motion.md — reduced-motion
  - typography.md — contrast and readability
---

# Accessibility

> **When to read this**: You're implementing focus management, ARIA attributes, keyboard navigation, or colour contrast — or you're doing an accessibility review.

Accessibility is product quality, not a compliance pass. If an interface is harder to perceive, navigate, or understand, it is worse software — full stop.

→ **Principles at work**: #3 (honest and predictable), #6 (feedback)
→ **Disabled button guidance**: `interaction.md`
→ **Reduced-motion**: `motion.md`
→ **Contrast context**: `typography.md`, `visual-design.md`

---

## Focus management

Use visible focus indicators on all interactive elements. The app uses a global `2px` **focus**-token ring with offset (`ring-focus ring-offset-canvas` in `src/app.css` `@layer base`).

Focusable elements in menus, option lists, and similar sequential collections must support arrow-key navigation.

Do not rebuild keyboard or focus behaviour from scratch. Use native HTML elements and established component libraries that already handle this well.

---

## Labels & roles

**Icon-only buttons must have an explicit `aria-label`.** There is no visual fallback for screen reader users.

**Decorative elements must use `aria-hidden="true"`.** Meaningful complex visuals must have an accessible name, summary, or supporting text.

**Images must use `<img>` tags with `alt` text.** This enables screen readers and right-click copy. Do not use CSS background images for meaningful content.

---

## Tooltips

Essential explanation belongs in the interface, not behind hover. These rules apply to all tooltip usage:

**Tooltips triggered by hover must not contain interactive content** (links, buttons). Users cannot reach them before they disappear.

**Disabled buttons must not have tooltips.** Disabled elements are removed from the tab order, so keyboard users will never trigger the tooltip or know why the button is disabled. Show an inline message instead.

**Do not put essential explanation in a tooltip.** If users need the information to understand the screen or use the feature correctly, show it inline.

Tooltips work best for: additional information that is helpful but not required, repeated object descriptions in dense interfaces, short labels or definitions for multiple similar objects.

---

## Colour & contrast

Do not rely on colour alone to convey meaning. Always pair colour with icons, text, or patterns.

Minimum contrast ratios (WCAG AA):
- **4.5:1** for body text
- **3:1** for large text (18pt+ or 14pt+ bold)

Test in both light and dark mode. The semantic colour tokens swap automatically via `data-theme`, but verify that specific combinations still meet contrast thresholds.

---

## Document-level

Use a `<svg>` favicon with a `<style>` tag that adapts to system theme via `prefers-color-scheme`.

Style the text selection state with `::selection` using brand or active colours.

---

## Quick reference: do / don't

| Do | Don't |
|---|---|
| Add `aria-label` to icon-only buttons | Rely on surrounding context for meaning |
| Use visible focus indicators | Remove outlines for aesthetics |
| Use `<img>` with `alt` for meaningful images | Use CSS background images for content |
| Show inline text explaining why a button is disabled | Put tooltips on disabled buttons |
| Pair colour with icons or text for meaning | Rely on colour alone |
| Support arrow-key navigation in lists and menus | Force users to Tab through every item |
| Respect `prefers-reduced-motion` | Assume all users want animation |
