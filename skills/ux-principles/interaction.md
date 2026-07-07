---
type: patterns
topic: interaction
when: Building a form, button, dropdown, toggle, filter, or any interactive element — or handling AI-generated data.
see-also:
  - principles.md — especially #2 (cognitive load), #5 (progressive disclosure), #6 (feedback), #8 (familiar patterns)
  - accessibility.md — focus management, disabled state guidance, tooltip rules
  - data-display.md — feedback placement for success/error states
---

# Interaction & Input

> **When to read this**: You're building any interactive element — form, button, dropdown, toggle, filter — or you're handling AI-generated or sensitive data in the UI.

These rules make software easier to learn, safer to use, and faster to trust. In ScribeFloat, the stakes are higher: errors are costly, feedback matters, and the main path should feel immediate.

→ **Principles at work**: #2 (cognitive load), #5 (progressive disclosure), #6 (feedback), #8 (familiar patterns)
→ **For spacing around inputs**: `visual-design.md`
→ **For ARIA and keyboard**: `accessibility.md`
→ **For success/error feedback placement**: `data-display.md`

---

## Forms & inputs

**Label clicks must focus the input.** Wrap inputs with `<label>` or use `htmlFor`. This is a basic web contract — don't break it.

**Wrap inputs in a `<form>`** so users can submit with Enter. It is still one of the best interaction patterns on the web.

**Use the correct `type` attribute**: `email`, `password`, `url`, `tel`. This gives users the right mobile keyboard and enables browser autofill.

**Use `inputMode` when `type` alone is not enough.** A date-of-birth field should use `type="text"` with `inputMode="numeric"` — you get the numeric keyboard without the spinner arrows from `type="number"`.

**Disable `spellcheck` and `autocomplete`** on inputs where they are unhelpful: codes, IDs, search queries. Leave them on for free-text fields.

**Use semantic validation attributes** (`required`), but provide clear inline validation messages where the product needs more control or clearer wording.

**Never block paste** on inputs or textareas. Users paste passwords, addresses, and codes constantly.

**Position prefix/suffix icons absolutely** on top of the input with padding, not adjacent to it. Disable `pointer-events` on decorative icons so clicking them focuses the input underneath.

**Let textareas grow with their content.** Use `field-sizing: content` with `min-height` and `max-height` constraints so textareas expand as the user types. Pair with `resize-none`.

---

## Buttons & actions

**Disable buttons after submission** to prevent duplicate requests. Show a loading spinner if the user is waiting for a network response. Users often double-click instinctively in fast workflows.

**Toggles take effect immediately** — no confirmation step. If something needs confirmation, it is not a toggle; it is a setting with a save button.

**Display success feedback relative to its trigger.** A successful copy should show a temporary checkmark on the button, not a toast in the corner. A form error should highlight the failing input, not the entire form.

**Optimistically update low-risk, reversible actions** and roll back on server error. For destructive, legal, confidential, or high-impact actions, show clear pending → confirmed → failed states instead.

**Confirm before destroying user effort.** Deleting something the user invested time creating (a document, draft, saved search) requires confirmation. Low-effort changes (filter selections) do not — they're quick to recreate.

**Cancel stale requests when the user moves on.** If a user starts a new search or AI query before the previous one finishes, abort the in-flight request immediately. Use `AbortController`. Do not queue them or make the user wait.

---

## Interactive elements

**Disable `user-select`** on button labels and interactive element content. Users clicking a button do not want to accidentally select its text.

**Decorative elements** (glows, gradients, overlays) must have `pointer-events: none` so they don't intercept clicks.

**Menus should feel immediate on pointer interaction.** Avoid delays that make the UI feel sluggish. Prefer established accessible menu components.

**Focusable elements in sequential lists** (search results, menu items, file lists) must support keyboard navigation with arrow keys.

---

## Filters

When multiple filters can be changed in quick succession and each change triggers a network request:

**Update the filter UI immediately, debounce the network call.** Reflect the change in the UI instantly, batch the API call behind a 500–800ms debounce.

**Bypass debounce for reset actions.** "Clear all filters" should fire immediately. Cancel any pending debounced call at the same time.

**Skip redundant requests.** Before applying debounced filters, compare the new state against the current active state. If nothing changed, don't fire.

**Only debounce when the cost justifies it.** If filters don't trigger network requests (client-side filtering on a small dataset), apply them immediately.

---

## Explanations, AI, and data handling

**Use hierarchy to guide the user through the main workflow.** Put the most common path front and centre; keep less common actions and settings available with less visual weight.

**Show important explanation in the interface.** If a user needs information to understand a feature or what will happen next, show it inline — not behind a tooltip or help modal.

**Use tooltips for extra help, not core meaning.** Tooltips are appropriate for additional information, repeated descriptions, or compact UIs where many items need short labels. See `accessibility.md` for tooltip rules.

**Clearly label AI-generated information.** If a summary, suggestion, classification, or draft is AI-generated, say so directly in the UI.

**Tell users what they still need to verify.** AI output should help them move faster, but the interface should make clear when they still need to check for inaccuracies.

**State the app's data clearance clearly.** Users should not have to guess whether they can put confidential conversations or meeting recordings into the product.

**Put data-handling guidance near the point of use.** If an input has restrictions, or if a workflow is approved only for certain data classes, show that where the user is making the decision.

---

## Quick reference: do / don't

| Do | Don't |
|---|---|
| Submit forms on Enter via `<form>` wrapping | Require users to find and click a submit button |
| Disable buttons after submission, show loading | Allow duplicate submissions on double-click |
| Show success feedback at the trigger location | Show a generic toast for everything |
| Make menus feel immediate | Add avoidable delay to frequent menu interactions |
| Support arrow-key navigation in lists | Force users to Tab through every item |
| Optimistically update low-risk reversible actions | Wait for server round-trip to show every low-risk change |
| Show important explanation inline | Hide core guidance in tooltips |
| Label AI-generated output clearly | Present AI output as if it were confirmed fact |
| State what data the app can handle | Make users guess what they're allowed to enter |
| Cancel stale requests when the user starts a new one | Queue requests or make users wait for the previous to finish |
| Let textareas grow with content via `field-sizing: content` | Force users into a fixed-height textarea with a scrollbar |
