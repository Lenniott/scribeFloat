# Commit Message Conventions

Use Conventional Commits.

## Subject

```text
type(scope): outcome
```

Keep the subject short and behavior-focused.

Preferred types:

- `fix`
- `feat`
- `refactor`
- `test`
- `docs`
- `chore`

## Body

Include a body when the reason is not obvious. The body should explain:

- user impact or review context
- root cause
- why this implementation is safe
- test or validation signal, when useful

Avoid bodies that only restate file edits.

## Production Example

```text
fix(onboarding): let Dictate drive practice input

The practice step mirrored Dictate DONE events into local textarea state,
which made onboarding behave differently from real Dictate usage and could
duplicate pasted text.

Keep the composer visible and let native paste/Enter update and submit the
textarea.
```

## Prototype Example

```text
prototype(onboarding): prove Dictate practice flow

Explores the designer-approved onboarding behavior where Dictate is tested
against a real textarea instead of a fake transcription state.

Production signal:
- DictatePracticeStep owns the teaching UI
- NoteComposer already provides Enter-to-submit behavior
- duplicate text came from local DONE-event mirroring
```

