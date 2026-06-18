---
type: checklist
topic: review
when: Reviewing any interface before merging or shipping.
see-also:
  - user-context.md — litmus test
  - All other files — this checklist synthesises every chapter.
---

# Pre-Ship Checklist

> **When to read this**: You're reviewing an interface before merge, handoff, or release. Run every section — they are there to catch obvious misses early.

---

## Hierarchy & structure

- [ ] Blur your eyes. Can you still tell what's most important?
- [ ] Are sections visually separated with consistent spacing?
- [ ] Does the text hierarchy use distinct levels (title → body → caption → muted)?
- [ ] Are related elements grouped? Are unrelated elements separated?
- [ ] Is the most important action the most prominent element?
- [ ] Is important information visible without forcing users to hunt through drawers, tabs, tooltips, or extra clicks?
- [ ] Does the visual hierarchy lead users through the main workflow first, while keeping secondary actions and settings available but quieter?
- [ ] Are familiar patterns used where they help users learn the product faster?

---

## States & feedback

- [ ] Does every interactive element have hover, focus, active, and disabled states?
- [ ] Does every data view handle loading, empty, and error?
- [ ] Do buttons show a loading state during async operations?
- [ ] Is success/error feedback shown at the point of action (not in a distant toast)?
- [ ] Are form errors highlighted on the specific failing input?
- [ ] Can a tired user move through the common workflow without friction, confusion, or unnecessary pauses?
- [ ] Is important explanation shown inline when users need it, rather than hidden in tooltips?

---

## Accessibility

- [ ] Can the entire interface be used with keyboard only?
- [ ] Do all icon-only buttons have `aria-label`?
- [ ] Is colour contrast at least 4.5:1 for body text and 3:1 for large text?
- [ ] Does the interface not rely on colour alone for meaning?
- [ ] Are focus indicators visible and clear?
- [ ] Is `prefers-reduced-motion` respected?
- [ ] Are tooltips only being used for additional information, not essential explanation?

---

## Responsive

- [ ] Does the layout stay usable at narrower widths, in split-screen, and in resized laptop windows?

---

## Dark mode

- [ ] Do surfaces still layer correctly (elevation increasing)?
- [ ] Are borders and dividers visible?
- [ ] Are overlays (modals, drawers, menus) distinguishable from their backdrop?
- [ ] Is text readable at all hierarchy levels?
- [ ] Are semantic colours (error, warning, etc.) legible?

---

## Performance

- [ ] Is the initial load fast? Are heavy resources lazy-loaded?
- [ ] Are animations using `transform`/`opacity` only?
- [ ] Are off-screen animations paused?
- [ ] Does the UI feel responsive, with feedback on actions in under 400ms?
- [ ] Does the main workflow feel snappy rather than slow, clunky, or overbuilt?
- [ ] Are loading states structural skeletons, not generic spinners?

---

## Typography

- [ ] Are only weights 400+ used?
- [ ] Is `antialiased` rendering enabled?
- [ ] Are numbers in `tabular-nums` where they appear in columns?
- [ ] Is long text handled with `truncate` or `line-clamp`?
- [ ] Are fonts subsetted for the relevant character set?

---

## Design consistency

- [ ] Is decorative colour use restrained? One **brand** CTA per view; **focus**, **active**, **warning**, **success**, and **destructive** only where semantically correct?
- [ ] Are all colours from semantic tokens, not raw values?
- [ ] Is border radius consistent across same-level elements?
- [ ] Does the spacing follow a 4px grid?
- [ ] Are the same patterns used for the same purposes throughout?
- [ ] Does the interface still feel like a ScribeFloat tool rather than a generic SaaS dashboard?

---

## AI & data handling

- [ ] Is AI-generated content clearly labelled wherever it appears?
- [ ] Is it clear what the user still needs to verify for accuracy?
- [ ] Is the app's data clearance clearly stated?
- [ ] Are data-handling restrictions shown near the point where users enter, upload, or generate sensitive information?

---

## The litmus test

> At 11pm, would a product designer open this without hesitation, expecting it to be fast, clear, and worth using? Does it feel neutral, trustworthy, and specific to knowledge work rather than like a generic SaaS product?

If the answer is "not quite":
- Check **hierarchy** — is the most important thing visually dominant?
- Check **density** — is there wasted space?
- Check **speed** — does every action feel immediate?
- Check **trust** — is the product being honest about what it's doing?

→ **Context for these checks**: `user-context.md`
→ **The principles behind each category**: `principles.md`
