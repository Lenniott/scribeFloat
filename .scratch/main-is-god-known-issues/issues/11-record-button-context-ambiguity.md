---
title: "Triage: Record button context: new note vs continue in note"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Record button context: new note vs continue in note" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- Single control, single handler: `src/lib/ui/6_regions/TitleBar.svelte:48-59` (`handleRecordClick`). There is only one Record button rendered by `TitleBar.svelte` (line 196: `<Button variant="normal" size="small" onclick={handleRecordClick}>Record</Button>`), which is a global region mounted once by the app shell (`src/routes/+layout.svelte:209-210`, `<ShellTitleBar onNewNote={openCapture} .../>`). There are not two separate components/call sites — it's the same instance whose behavior branches at click time.
- Branch logic (`TitleBar.svelte:48-59`):
  ```
  function handleRecordClick() {
    if (scribe.phase !== 'idle') return;
    const match = page.url.pathname.match(/^\/notes\/([^/]+)$/);
    const noteId = match?.[1];
    if (noteId && noteId !== 'new') {
      void scribe.startRecording(noteId);   // continue-in-note path
      return;
    }
    if (!onNewNote) return;
    appState.scribeAutoStart = true;
    onNewNote();                            // new-note path
  }
  ```
  The decision is purely based on the current route (`/notes/:id` with a real id vs. anywhere else, e.g. `/notes/new`, Home, Upload). Label text is a static `"Record"` (line 196) — no conditional copy today.
- `isOnRecordingNote` (line 43-46) already computes "am I looking at the note currently being recorded into" for a different purpose (showing/hiding the "go to note" affordance at lines 109-124), so the plumbing to detect context already exists and could be reused to drive a distinct label/tooltip.
- What a fix would touch: `TitleBar.svelte` only for a label/tooltip change (e.g. swap `"Record"` for `"Record into note"` vs `"New recording"` based on the same route match used in `handleRecordClick`, or add an `aria-label`/title). No store or IPC changes needed since `scribe.startRecording` / `onNewNote` already encode the two behaviors correctly — this is a discoverability/labeling gap, not a logic bug.
- A confirmation-dialog variant (per the "Later" note's second option) would be a larger change: needs a new Modal state, likely reusing the existing `showDiscardConfirm` / `Modal` pattern already in this file (lines 35, 212-213) as a template.
- Size estimate: label/tooltip-only fix is small (~1 file, under an hour). A confirmation-modal fix is small-medium (new modal state + wiring, still confined to `TitleBar.svelte`, a few hours).
