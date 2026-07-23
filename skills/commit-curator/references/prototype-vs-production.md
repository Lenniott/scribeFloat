# Prototype vs Production

Choose commit style based on the branch purpose.

## Exploration / Prototype Branch

Use when the branch is design-led, investigatory, or intended to help developers understand a direction.

Commit bodies should explain:

- design behavior explored
- files/components that proved relevant
- assumptions or shortcuts
- production follow-up questions

Review maps should surface insight, not only correctness.

## Production Branch

Use when the branch is intended to merge.

Commit bodies should explain:

- bug or feature impact
- root cause
- implementation choice
- tests proving behavior

Avoid speculative language unless the code intentionally leaves a follow-up.

## Handoff Guidance

For designer prototypes, include developer review notes:

- which components own the interaction
- which state paths are duplicated
- which UX behavior should be kept even if implementation changes
- which code is likely throwaway

