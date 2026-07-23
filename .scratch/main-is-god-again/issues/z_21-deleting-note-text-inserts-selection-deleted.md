---
title: Deleting note text inserts Selection deleted
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-agent
blocked_by: []
parent: MAP.md
---

## Question

When a user deletes part of the written note body, does the editor stay clean — or does junk like “Selection deleted” appear in the note content?

**Done when:** Selecting and deleting (or backspacing a range of) written note text never inserts “Selection deleted” (or similar chrome/status copy) into the document; note body stays what the user typed.

## Why merge-blocker

Ship-bar Notes step requires opening a Note with sane content. Corrupting the written pane on ordinary delete fails the map’s “capture + Notes still work” confidence bar. Human marked blocker during Silicon smoke (2026-07-21).

## Seen

Installed Silicon `.app`, Note editor Written pane. After deleting a selection, the literal string **Selection deleted** appeared at the top of the note area (inside the editor chrome / content). Recording / transcript side looked fine.

Status: ready-for-agent

## Spec (to-spec)

### Problem Statement

In the Note editor Written pane, selecting text and deleting it can show the literal string **Selection deleted** at the top of the editor. That string is not app copy and must not appear as if it were note content. Ship-bar Notes confidence fails when ordinary editing looks corrupted.

### Solution

Keep delete as a pure content mutation. Hide or contain CodeMirror’s accessibility announce live region (`.cm-announced`) in the Written pane so “Selection deleted” never appears as editor chrome, and ensure the note document / autosaved written content never contains that string.

### User Stories

1. As a Notes user, I want select → delete to remove only my selected text, so that the Written pane stays trustworthy.
2. As a Notes user, I want select → backspace/delete to never show “Selection deleted” at the top of the editor, so that chrome does not look like document text.
3. As a Notes user, I want autosave after a range delete to persist only my remaining words, so that relaunch does not revive junk copy.
4. As a Notes user editing markdown (headings, bold), I want normal CodeMirror editing to keep working, so that the fix does not gut the Written pane.
5. As a VoiceOver / screen-reader user, I want delete announcements handled safely (hidden visually or replaced carefully), so that we do not “fix” visibility by breaking a11y without thought.
6. As a Silicon `.app` user (WKWebView), I want the announce region hidden even if CodeMirror’s default off-screen positioning fails, so that the smoke bug cannot return on installed builds.
7. As a Chromium / `npm run dev` user, I want the same editor to remain clean, so that dev and shipped UI match.
8. As a user undoing a delete, I want undo to restore my text, not announce chrome, so that history stays about the document.
9. As a maintainer, I want the fix localized to `MarkdownEditor` (theme/CSS/extensions), so that note autosave and history controllers stay unchanged unless a doc leak is proven.
10. As an agent writing tests, I want a regression that select-delete does not put “Selection deleted” into `doc` / onchange payload, so that we do not rely only on manual smoke.
11. As a reader of ADR-0008, I want this ticket to keep CodeMirror as the Written source editor, so that we do not reopen TipTap.
12. As a ship-bar tester, I want open Note → select written text → delete → no “Selection deleted” visible, so that ticket 09’s Notes fail is cleared.
13. As a user with an empty selection delete (single character backspace), I want no spurious announce chrome either, so that all delete paths stay clean.
14. As someone investigating future editor bugs, I want the Resolution to name `.cm-announced` / `EditorView.announce` as the source, so that we do not re-hunt TipTap myths.

### Implementation Decisions

- **Primary seam:** `MarkdownEditor` (CodeMirror 6) theme / base styling for `.cm-announced`. Not TipTap/ProseMirror (ADR-0008).
- **Root cause (confirmed):** `@codemirror/commands` range delete dispatches `EditorView.announce.of(state.phrase("Selection deleted"))`. The live region is a sibling of `.cm-scroller` inside `.cm-editor`. Default CM theme parks it at `position: fixed; top: -10000px`. Browser repro (2026-07-23): announce text is present; with off-screen positioning disabled it sits at the **top of the editor** — matching Silicon smoke. Document (`cm-content`) did not contain the string.
- **Fix:** In `MarkdownEditor`’s `EditorView.theme` (or equivalent CSS), force `.cm-announced` to remain non-visible in layout (e.g. keep/strengthen off-screen or `sr-only`-style clipping). Prefer hardening theme over removing announce entirely; if announce is removed, note a11y trade-off in Resolution.
- **Autosave:** `onchange` / `updateListener` already reads `doc`; verify it never receives announce text. No HistoryController change unless a leak is found.
- **Do not** filter the string out of user documents as a content ban-list (user could type those words); fix the chrome visibility instead.

### Testing Decisions

- Good tests assert **external behaviour**: after a select-all + delete (or range delete) in `MarkdownEditor`, the document string / onchange value does not equal or contain announce chrome as content; ideally the announce node is not visually laid out in the editor box.
- **Modules:** add or extend Vitest around `MarkdownEditor` if practical under jsdom; otherwise a focused theme assertion + manual Silicon/dev check.
- **Prior art:** ADR-0008; note leave/autosave tests; no existing `MarkdownEditor.test.ts` — creating a small one is OK.
- Browser evidence: Chromium hides announce with CM defaults; forcing `position: static` reproduces visible “Selection deleted” at editor top; doc stayed clean.
- `npm test` (relevant) + manual Written-pane delete on Silicon or `tauri dev`; `cargo test -p ScribeFloat` if Rust untouched should still pass in CI habit.

### Out of Scope

- Note pane height / full-height layout (Known issues)
- Speaker rename this-vs-all (Known issues)
- Focus ring styling (Known issues)
- Replacing CodeMirror with another editor
- Broad a11y audit of the Note editor beyond this announce leak

### Further Notes

- Evidence: Silicon smoke 2026-07-21; code + browser mimic 2026-07-23.
- Ticket lives as merge-blocker (not Known issues) per HANDOFF.

## Resolution

Implemented 2026-07-23. Hardened `.cm-announced` in `MarkdownEditor` `EditorView.theme` with sr-only clipping (source: CodeMirror `EditorView.announce` / `@codemirror/commands` “Selection deleted”). Document/`onchange` never contained the string; chrome visibility was the bug. Vitest asserts clean doc after range delete + announce node styling.
---
