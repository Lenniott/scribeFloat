---
id: "0043"
title: Add behaviour tests for UI components
status: active
---

# Add behaviour tests for UI components

The codebase has Vitest + @testing-library/svelte configured. Covered today:

- `DictatePracticeStep.test.ts` — onboarding dictate step
- `WelcomeStep.test.ts`, `ModelDownloadStep.test.ts`, `PermissionsStep.test.ts`, `FeatureTourStep.test.ts`, `onboarding.test.ts` — onboarding wizard
- `scribeController.test.ts`, `dictate.test.ts` — capture state machines (mocked IPC)
- `noteLeaveGuard.test.ts` — note editor leave-guard decision tree (0049)
- `Button.test.ts`, `Toggle.test.ts`, `NoteCard.test.ts`, `FilterRow.test.ts`, `Accordion.test.ts` — primitives (this story)
- `modelDownload.test.ts`, `types.test.ts`, `captureProgress.test.ts`, `processingFeedback.test.ts`

Shared fixtures: `src/test/ipcFixtures.ts` (history/model mocks, event bus helpers).

`vitest.config.ts` aliases match `svelte.config.js` so component tests resolve `@patterns`, `@sections`, etc.

## What to cover next

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
