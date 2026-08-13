# Notes metadata / tags panel

Parked from [main-is-god-known-issues](../../.scratch/main-is-god-known-issues/MAP.md) ticket 15. Hold until Note processing / Float features that need tags exist — not worth stripping or finishing in isolation.

## Summary
- Difficulty: **small to strip**, **small-medium to finish properly**.
- Note editor has a working "Metadata" toolbar toggle that opens onto placeholder copy: "Tags and keywords — not wired yet."
- No tags field on the note data model, no persistence, no commands.

## Why it's parked here
Building tags now invents a feature without the processing/Float consumers that make it useful. Stripping the panel is trivial but also discards chrome we expect to want once those features land. Hold.

## Research already done
Grounded findings live in the closed triage ticket:
`.scratch/main-is-god-known-issues/issues/z_15-unfinished-notes-ui-leftovers.md`

Key anchors:
- UI: `src/lib/ui/5_views/note-editor.svelte` — `rightPanelOptions` + `showMetadata` branch
- Strip path: remove Metadata from `RightPanel` / toggle / panel block (~15–20 lines)
- Finish path: tags on history/note record + persistence + UI (touches history service / sidecar)

## Suggested future destination
When Float/processing needs tags or keywords, chart a Notes-metadata wayfinder that designs the data model first, then replaces the placeholder panel — do not wire a half feature just to clear the chrome.
