---
id: "0070"
title: Per-note button to remove voice embeddings from the current note
status: active
---

# Per-note button to remove voice embeddings from the current note

As a privacy-conscious user, I want a button on a note that removes the voice
embeddings stored in that note only — not a bulk wipe — so that the global
voiceprint profiles (the "average print") keep working while this particular
recording holds no biometric vectors.

The backend already exists and is per-note:

- IPC `note_remove_voice_embeddings(id)` in `src-tauri/src/commands/history.rs`
- `HistoryService::remove_voice_embeddings()` strips chunk embeddings,
  encrypted vectors, and session-speaker centroid embeddings while keeping
  transcript text, speaker labels, timing, quality scores, cuts, chunks, and
  session speaker groups
- Global voiceprint profiles live in the profile store and are untouched

What is missing is the UI:

- A "Remove voice data from this note" action on the note editor (natural home:
  the metadata sidebar, story 0047), with a confirm step — the removal is
  irreversible for this note
- The action should only be visible/enabled when the note actually carries
  embeddings (chunk `embedding`/`encrypted_embedding` or session-speaker
  centroids present)
- After removal, reflect the scrubbed state (e.g. the action swaps to a
  "No voice data stored" hint) without needing a reload

## Notes

- Do not touch `history_remove_all_voice_embeddings` (bulk) — that stays in
  Settings; this story is the per-note affordance only
- Removing embeddings disables future re-scoring/corrections for this note
  (stories 0061–0063 need the vectors); the confirm copy should say so
