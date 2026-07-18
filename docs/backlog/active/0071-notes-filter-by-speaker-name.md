---
id: "0071"
title: Filter Notes list by speaker name
status: active
adr: ADR-0014
---

# Filter Notes list by speaker name

As a user with many recorded notes, I want to filter the Notes list to only those featuring a given person, so I can find "everything with Sarah" without opening each note.

This was the deferred follow-up from the ADR-0014 diarization work: live Sortformer diarization plus a plain `speaker_names.json` store (`services/speaker_names.rs`) replaced the voiceprint engine. Speakers are anonymous per-note (`Speaker 1`–`4`) until relabeled via `note_relabel_speaker`, at which point the name is also saved globally for reuse across notes — but nothing today aggregates "which notes mention Ben" for filtering.

## Scope

- Extend `HistoryListItem` (backend `types.rs`, frontend TS mirror) with a derived `speaker_names: Vec<String>` — the set of non-reserved `speaker_blocks[].label` values on that note (reuse `speaker_names::is_reserved_speaker_label` to exclude `Other`/`In`/`Out`/auto-assigned `Speaker N` labels that were never renamed).
- Add a `history_speaker_name_vocabulary` command: distinct speaker names across all live (non-deleted) notes in the save folder, for populating filter options. Counts live notes only.
- Add a Speakers section to `FilterPanel.svelte`, combined with existing capture-method and tag filters using AND semantics (matches the pattern the other filter categories already use).
- Since relabeling is per-note and matches by label string (ADR-0014's known limitation — two historically distinct speakers sharing a label merge), the filter is inherently best-effort for old chunk-based notes; that's acceptable and doesn't need special-casing.

## Notes

- No new IPC for name aggregation beyond the one vocabulary command — `speaker_names_list` (existing) already gives the full name catalog if the UI wants to show even zero-note names.
- Anonymous `Speaker N` labels that were never renamed should NOT appear as filter options — only names a user has actually assigned.
- Depends on nothing else outstanding; the underlying data (`speaker_blocks` on `HistoryRecord`) already exists and is populated by both Record and Upload today.
