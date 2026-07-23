---
title: Deleting note text inserts Selection deleted
labels: [wayfinder:task, needs-triage]
status: open
assignee:
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

## Likely fix direction (not to-spec yet)

Find the source of the string (editor plugin, accessibility live region leaking into ProseMirror/TipTap doc, undo toast mis-wired as content). Stop writing UI status into the note document; keep delete as a pure content mutation.

## Out of scope here

- Note pane height / full-height layout (Known issues)
- Speaker rename this-vs-all (Known issues)
- Focus ring styling (Known issues)
---
