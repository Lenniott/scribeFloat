---
type: patterns
topic: motion
when: Adding any transition, animation, or hover effect — or reviewing whether motion is justified.
see-also:
  - principles.md — #1 (respect time), #4 (real context)
  - accessibility.md — reduced-motion implementation
  - visual-design.md — performance and compositing rules
---

# Motion & Animation

> **When to read this**: You're adding a transition, animation, or hover effect — or you're reviewing whether an existing animation is justified.

Good motion explains change. It should show state, direction, and feedback without slowing people down. In ScribeFloat, motion is functional, not decorative.

→ **Principles at work**: #1 (respect time — never add latency), #4 (real context — frequent tools need zero-novelty interactions)
→ **Compositing rules** (why `transform`/`opacity` only): `visual-design.md`
→ **Reduced-motion implementation**: `accessibility.md`

---

## Timing

Interactions should feel immediate. Anything over 150ms for direct manipulation (hover, press) will feel sluggish.

| Duration | Use case |
|---|---|
| 100–150ms | Micro-feedback: hovers, button presses, toggles |
| 200–300ms | Small transitions: dropdowns, tooltips, tabs |
| 300–500ms | Medium transitions: drawers, modals, panels |
| > 500ms | Almost never. If it's this slow, question whether you need animation at all. |

---

## Scale & proportion

Animation values should be proportional to the element's size. Small elements need small motion.

**Dialog/modal**: fade opacity + scale from `~0.95`, not from `0 → 1`. Scaling from zero looks cheap and is visually extreme.

**Button press**: use `~0.96` or `~0.98`, not `~0.8`. Users pressing a button don't need a dramatic press animation.

**Tooltip/popover**: translate `4–8px` from the anchor, not `50px`. The user's eye is already near the trigger.

---

## When to skip animation entirely

Some interactions are better with no motion at all. Frequent, low-novelty actions should be instant:

- Opening a context menu or right-click menu
- Adding or removing items from a list
- Hovering trivial or utility buttons
- Switching tabs in a familiar interface

The native macOS context menu only animates *out*, not *in*, because it gets used so often. The same logic applies here. Animation should serve the user, not the developer.

---

## Purpose taxonomy

If an animation does not serve one of these purposes, remove it.

| Purpose | Example |
|---|---|
| **Confirm** | Button press feedback, save confirmation |
| **Orient** | Drawer sliding in from the edge, dropdown expanding |
| **Focus** | Error state drawing attention, new item highlighting |
| **Connect** | Tab content crossfading, accordion expanding in place |

---

## Reduced motion

Always respect `prefers-reduced-motion`. People usually set it for a medical reason. Disable transitions and animations when this media query is active.

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

→ See also: `accessibility.md` for full reduced-motion context.

---

## Quick reference: do / don't

| Do | Don't |
|---|---|
| Keep direct interactions under 150ms | Animate everything because it looks polished |
| Scale proportionally: `0.96`, not `0.5` | Scale dialogs from `0` or buttons to `0.8` |
| Skip animation on frequent, low-novelty actions | Add entrance animation to context menus |
| Respect `prefers-reduced-motion` | Assume all users want motion |
| Animate only `transform` and `opacity` | Animate `width`, `height`, `margin`, `top` (causes reflow) |
| Translate tooltips `4–8px` from anchor | Slide tooltips in from `50px` away |
