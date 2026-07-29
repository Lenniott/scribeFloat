---
title: "Triage: Unfinished Notes UI leftovers"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

The note editor's "Metadata" side panel is a fully wired, working toggle button that opens onto a single placeholder line ("Tags and keywords — not wired yet.") — dead-end chrome with zero backing feature (no tags field on the note data model, no storage, no commands).

## Question

Read the "Unfinished Notes UI leftovers" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now**, trivial — strip the placeholder panel and toggle entirely (net simplification, nothing else depends on it)? Or is tags/keywords a real feature worth building properly instead (small-medium, touches `HistoryRecord` + persistence)?

## Findings

- Confirmed and reproduces as written: `src/lib/ui/5_views/note-editor.svelte:247-252` — the "Metadata" side panel (toggled via the button built from `rightPanelOptions` at line 47-50, label "Metadata") renders only a single placeholder line: `<p class="sf-body-md text-fg-muted">Tags and keywords — not wired yet.</p>`. No tags/keywords data model, store, or backend command backs this panel anywhere in the file or its imports.
- This is pure dead-end chrome: a real toggle button in the toolbar (line 210-224) that is fully functional (state-managed via `showMetadata`/`showTranscript`, mutually exclusive with Transcript per `toggleRightPanel` at line 61-75) but opens onto placeholder text with zero backing feature.
- To strip: remove `'metadata'` from `RightPanel` type (line 16), `rightPanelOptions` entry (line 49), the `showMetadata` state/branches in `defaultRightPanels` (52-59) and `toggleRightPanel` (61-75+), and the `{:else if showMetadata}` block (247-252) — leaving Written+Transcript as the only pane pair. This is a net simplification, not a risky change, since nothing else references `showMetadata`.
- To finish cheaply instead: would need an actual tags/keywords field on the note record (no `tags` field currently seen on `HistoryRecord` type at line 18-23) plus backend storage/commands — this is a real (if small) feature, not a copy fix, so "finish cheaply" is relative.
- Size estimate: trivial to strip (delete ~15-20 lines, one type/array edit, no backend touch); small-medium to actually finish (new field on HistoryRecord + persistence + simple UI, backend touches `src-tauri/src/services/history.rs` and note sidecar/save path).
