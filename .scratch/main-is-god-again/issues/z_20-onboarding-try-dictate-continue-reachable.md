---
title: Onboarding Try Dictate Continue reachable
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-agent
blocked_by: []
parent: MAP.md
---

## Question

On the onboarding “Try Dictate” step, can a user always reach Continue after a successful practice capture — including long pasted/dictated text — without an opaque “send N more messages” gate trapping them?

**Done when:** After at least one successful practice Dictate (or an explicit Skip), Continue stays visible and usable; long transcript text cannot push footer actions off-canvas; any practice gate is obvious in the UI (or removed).

## Why merge-blocker

First-run includes finishing Setup. Human hit a long Dictate during practice: history overflow buried / removed Continue; progress only returned after several more short sends (suspected ~4-message gate). That is a real trap on the ship-bar first-run path, not polish.

## Seen

Silicon ship-bar smoke onboarding Try Dictate (2026-07-21). Short capture worked; large text filled the preview and Continue disappeared until more messages were sent.

Status: ready-for-agent

## Spec (to-spec)

### Problem Statement

On Try Dictate, a long practice note makes Continue unreachable. Browser repro (2026-07-23) showed Continue stays enabled in the DOM, but the tall practice `NoteCard` overflows the history region and **covers** the footer — `elementFromPoint` on Continue hits the note text instead. There is no silent multi-send gate in code (`MAX_NOTES = 2`; Continue is never disabled). Short follow-up notes shrink the card and accidentally “restore” Continue, which felt like a message-count gate during smoke.

### Solution

Treat oversized practice text as an edge case: **clamp** practice-note display (or stored practice preview text) to a character limit and append `...` when truncated, so the card cannot grow tall enough to cover Back / Continue. No Continue gating, no Skip requirement, no full layout redesign for this ticket.

### User Stories

1. As a first-run user who dictates a very long practice utterance, I want Continue still clickable, so that I can finish Setup.
2. As a first-run user who pastes a huge block into the practice composer and sends it, I want the preview clamped with `...`, so that the footer is not buried.
3. As a first-run user with a short practice note, I want the full text shown (no needless ellipsis), so that normal practice is unchanged.
4. As a first-run user, I want Continue always available on this step (as today), so that I am not blocked by a hidden success counter.
5. As a first-run user who never sends a practice note, I want Continue still available, so that practice is optional for merge confidence.
6. As a first-run user, I want Back to keep working after a long clamped note, so that both footer actions stay usable.
7. As a first-run user with two practice notes (the existing cap), I want clamping applied per note, so that two medium notes cannot recreate the cover-up.
8. As a user reading a clamped card, I want a clear `...` suffix, so that I understand text was truncated in the preview.
9. As a Silicon smoke tester, I want one long practice send to leave Continue clickable without sending more messages, so that the false “gate” story is gone.
10. As a maintainer, I want the clamp limit documented in the Resolution, so that future agents do not re-litigate layout vs clamp.
11. As a design-system / Vitest author, I want a unit test that a long string becomes clamped with `...` and that Continue remains the hit target (or at least not covered by note height assumptions), so that the edge case cannot regress quietly.
12. As a user whose real Dictate paste into another app is long, I want this clamp to affect **onboarding practice preview only**, so that product Dictate output is not truncated.
13. As an accessibility user, I want Back / Continue to remain reachable without scrolling under a giant card, so that keyboard/pointer users are not trapped.
14. As a reader of Known issues, I want practice timestamp weirdness and double-tap gamification left parked, so that this ticket stays narrow.

### Implementation Decisions

- **Primary seam:** `DictatePracticeStep` practice notes / `NoteCard` display for onboarding practice only.
- **Agreed approach (human 2026-07-23):** clamp beyond a character limit with trailing `...`. Edge-case fix — not footer pinning, not overflow redesign, not Continue-after-N-success gating.
- **Suggested limit:** ~400 characters of visible practice text per card (tune if a quick layout check needs it). Apply when adding a practice note and/or when rendering the card in this step so height stays bounded.
- **Do not** change Dictate paste/clipboard behaviour outside onboarding practice.
- **Do not** add a silent multi-send requirement; Continue remains always enabled.
- **Skip button:** not required for this ticket given Continue stays available.
- **MAX_NOTES = 2** may remain; clamp is what stops cover-up.
- Prefer a tiny pure helper (e.g. `clampPracticePreview(text, limit)`) for easy Vitest coverage.

### Testing Decisions

- Good tests assert **external behaviour**: input longer than the limit renders with `...` and length ≤ limit + ellipsis; short input unchanged; Continue is not disabled by long input.
- Prefer a hit-test or height assertion only if cheap in Vitest/jsdom; otherwise clamp unit test + manual onboarding window check at 680×560 with a long send.
- **Modules:** `DictatePracticeStep` / helper; extend `DictatePracticeStep.test.ts`.
- **Prior art:** existing DictatePracticeStep Vitest (auto-enter, DONE not copied into textarea).
- Browser repro evidence (2026-07-23): long note ~1176px tall covered Continue at footer.
- `npm test` (relevant), `cargo clippy` if Rust untouched may be N/A; do not break `cargo test -p ScribeFloat`.

### Out of Scope

- Gamifying double-tap vs tap-and-hold (Known issues)
- Weird practice timestamp display (Known issues)
- Cold Whisper preload on first practice (Known issues)
- Pinning StepFrame footer outside ScrollBody as a general layout project
- Adding Skip or gating Continue on successful capture
- Changing real Dictate output length

### Further Notes

- Smoke misread “send more shorts” as a gate; code has no such gate — overflow cover + `MAX_NOTES` replacement explained the recovery.
- If clamp alone ever proves insufficient (e.g. pathological line breaks), a follow-up may add `overflow-y-auto` on the history region — not in scope unless clamp fails verification.

## Resolution

Implemented 2026-07-23; **revised same day**. Character clamp (400 + `...`) was insufficient for multi-line / markdown-ish practice text. Practice notes now use **CSS `line-clamp-2`** via `NoteCard` `maxLines={2}` (visual height of the text node, not character count). History region is `overflow-hidden` + scroll so cards cannot cover the composer / Continue. Full text stays in the note; display only is clipped.
---
