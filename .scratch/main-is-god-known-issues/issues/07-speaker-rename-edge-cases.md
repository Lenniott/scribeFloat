---
title: "Triage: Speaker rename edge cases"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Speaker rename edge cases" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- Rename-all cascade confirmed working: `relabel_speaker_blocks()` in `src-tauri/src/services/history.rs:308-323` iterates `speaker_blocks`, `speaker_chunks`, and `session_speakers`, renaming every entry whose label equals `from` — this is the single write path, no per-occurrence variant exists.
- Call chain: `src/lib/ui/4_sections/TranscriptPanel.svelte:163-180` (`applyRelabel`) invokes Tauri command `note_relabel_speaker` (`src-tauri/src/commands/history.rs:204`) → `HistoryController::relabel_speaker` (`src-tauri/src/controllers/history.rs:106`) → `HistoryService::relabel_speaker` (`src-tauri/src/services/history.rs:137`) → `relabel_speaker_blocks`.
- The UI's `applyRelabel(block, label)` only ever passes `block.label` as `fromLabel` — it already has the specific block in hand but the backend rename is keyed purely by label string, not by block/chunk id, so it always renames every matching turn. There is no plumbing anywhere (frontend or backend) for "this occurrence only."
- To add single-occurrence rename: backend would need a variant keyed by `chunk_id`/block index rather than label (e.g. a new command or an optional `scope: 'one' | 'all'` param plumbed through `HistoryService::relabel_speaker` down to a new single-block mutation), plus a small UI affordance (e.g. two buttons or a toggle in the existing correction popover at `TranscriptPanel.svelte:235-260`) to pick scope before calling `applyRelabel`.
- Size estimate: small-medium — backend logic is a straightforward addition (new function alongside `relabel_speaker_blocks` that matches on `chunk_id` instead of label, plus a new Tauri command or optional param), UI change is a couple of buttons in the existing popover. Needs tests per repo's TDD convention (existing tests at `src-tauri/src/services/history.rs:426-497` and `src-tauri/src/controllers/history.rs:631-670` show the pattern to extend).
