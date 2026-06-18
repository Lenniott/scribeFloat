---
id: "0043"
title: Add behaviour tests for UI components
status: active
---

# Add behaviour tests for UI components

The codebase has Vitest + @testing-library/svelte configured and one example test (`DictatePracticeStep.test.ts`). No other components have tests. Add behaviour-focused tests (not snapshots) for the components most likely to regress: interactive controls, card actions, and the Accordion pattern.

## What to cover

**ui/controls/** — Button, IconButton, Toggle, OptionGroup
- Renders with correct variant/size class
- Click fires callback; disabled state blocks it
- Toggle emits `onchange` with toggled value

**ui/cards/** — NoteCard, FilterRow
- NoteCard: renders title/chip, fires `onselect` on click, fires `oncopy`/`ondelete` from icon actions; delete action absent when `ondelete` is undefined
- FilterRow: renders label, fires `ontoggle` with tag + checked value

**patterns/Accordion**
- Only one AccordionRow open at a time
- Clicking an open row closes it

## Notes

- Use `@testing-library/svelte` render + `screen` + `fireEvent` / `userEvent`. No snapshot tests.
- Mock `@tauri-apps/api/core` and `@tauri-apps/api/event` where needed (see existing setup in `src/test/setup.ts`).
- Test files live next to the component: `Button.test.ts` beside `Button.svelte`.
- Do not test visual appearance (colours, classes) beyond what is needed to verify a variant prop wires up correctly.
