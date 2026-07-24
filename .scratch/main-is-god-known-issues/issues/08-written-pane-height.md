---
title: "Triage: Note written pane does not fill editor height"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Note written pane does not fill editor height" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- Layout chain in `src/lib/ui/5_views/note-editor.svelte:227-237`: outer row is `flex min-h-0 flex-1 overflow-hidden`; the Written column is `flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden`; its inner wrapper is `min-h-0 flex-1 overflow-y-auto` and hosts `<MarkdownEditor>`.
- `MarkdownEditor.svelte` root div is `h-full w-full` (last line of file), and its CodeMirror theme (`theme` object, ~line 55) sets `"&": { height: "100%", ... }` on `.cm-editor`. This is the standard "fill parent" pattern and looks structurally correct — no bordered/fixed-height box in the current code.
- No `border` or explicit `height`/`max-height` rule was found on `.cm-editor`/`.cm-scroller` in `app.css` (only a `.cm-editor :focus-visible` rule at app.css:343) that would cap it to a "short bordered box."
- Given the ticket's evidence date (2026-07-21, Silicon smoke) and that `note-editor.svelte`'s last functional layout commit is `ddd6ae0` (CodeMirror written source editor) with later commits (`1640704`, `ba56f8f`, `bbfee2a`, `c6df606`) not obviously touching this pane's box model, this may already be fixed, or the bug is more subtle (e.g. only reproduces with certain content/viewport sizes, or was screenshot before a later fix). Could not visually reproduce via static code read — needs a live run to confirm.
- If still reproducing live: fix would touch only `note-editor.svelte`'s flex classes and/or `MarkdownEditor.svelte`'s root/theme height rules — no data-model or backend changes.
- Size estimate: trivial-to-small if a real regression is found (CSS-only); zero-effort (close as already-fixed) if it does not reproduce — recommend a quick live screenshot check before scoping further.
