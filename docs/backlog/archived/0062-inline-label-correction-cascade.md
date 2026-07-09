---
id: "0062"
title: Inline label correction with centroid recalculation and cascading relabel
status: done
---

# Inline label correction with centroid recalculation and cascading relabel

As a user fixing a wrong speaker label in a transcript, I want correcting one line to automatically fix other lines that sound the same so that I don't have to hunt through the whole transcript manually — and I never need to go to Settings to do it.

Allow the user to correct a speaker label directly on a transcript segment without going to Settings. When a correction is applied:

1. Update the segment's `label` to the chosen speaker
2. Move the segment's embedding from the old label's group to the new label's group
3. Recompute centroids for both affected labels (story 0060 logic, same session)
4. Re-score all segments in the session against the updated centroids (story 0061 logic)
5. For any segment whose `session_score` has dropped below 0.75 or whose winning centroid has changed — auto-relabel and mark as `auto_corrected: true`
6. Persist the updated blocks and centroids to the session JSON

The cascading relabel in step 5 means fixing one `Other → You` can pull in several nearby segments that were borderline — the centroid shifts toward the corrected cluster and previously sub-threshold segments may now pass.

UI entry point: a tap/click on a speaker label chip opens a picker listing enrolled profiles + "Other". No settings navigation required.

## Notes

- Depends on stories 0059, 0060, 0061
- Auto-relabeled segments should be visually distinct from user-corrected ones so the user can review the cascade
- Do not auto-relabel segments with `margin < 0.05` — those are genuinely ambiguous and should stay as-is or prompt the user
- Correction history (original label, corrected label, timestamp) should be stored on the segment for auditability
- Story 0063 (global profile update) is a natural follow-on after correction, but keep them separate

## Completion note (2026-07-09)

Implemented on the chunk model: `correct_chunk_label()` in
`src-tauri/src/services/speaker_chunks.rs` moves the chunk, rebuilds the two
affected session-speaker centroids, re-scores every chunk, then cascades —
auto-relabeling chunks whose winning centroid changed, gated by
`margin >= 0.05` and never overriding explicit user corrections. Correction
history lives on `SpeakerChunk.corrections` (`LabelCorrection { from, to,
at_ms, auto }`); blocks follow chunks via `chunk_id`. IPC:
`note_correct_chunk_label` returns the updated record. UI: speaker chip in
`TranscriptPanel.svelte` opens a picker (profiles + session speakers + Other +
new name); auto-corrected blocks are marked distinctly from user-corrected.
