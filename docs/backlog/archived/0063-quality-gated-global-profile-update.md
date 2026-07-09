---
id: "0063"
title: Quality-gated global voiceprint profile update from transcript evidence
status: done
---

# Quality-gated global voiceprint profile update from transcript evidence

As a user who records meetings regularly, I want each good transcript speaker label to improve how well the app recognises that person over time so that I don't have to manually re-enrol people or go to Settings to keep accuracy high.

When a user names or confirms a transcript speaker, use the whole same-speaker group as evidence. Do not learn from only the clicked chunk. Build one transcript speaker centroid from the clean chunks in that group, then offer to add that centroid to the named global profile.

Global profiles should be rebuildable. Store accepted transcript-speaker evidence on the profile, then recalculate the global centroid, radius, and exemplars from all accepted evidence. Do not rely only on a rolling average that forgets its source records.

Only add transcript evidence when all three quality gates pass:

**Audio quality** (from stored signals on `SpeakerChunk`, story 0059):
- Clean speech duration across the speaker group >= 6 s
- Mean VAD purity >= 0.80
- No clipped chunks in the speaker group
- Prefer longer clean chunks over short chunks when computing the centroid

**Cluster quality** (from transcript speaker centroid, story 0060):
- `mean_score >= 0.85` — clean chunks agree with each other
- `std_dev <= 0.06` — tight cluster, not a mixed speaker group
- `radius` stays within the profile's accepted range, if the profile already exists

**Discriminability** (from margin scores, story 0061):
- Mean `margin` across clean chunks >= 0.15 — clearly separated from other session speakers
- Global profile match beats the next closest saved profile by the configured margin

If all gates pass, store an accepted evidence item on the named profile:

```text
record_id
session_speaker_id
centroid_embedding
clean_chunk_ids
duration_ms
quality_score
radius
accepted_at
```

Then rebuild:

```text
profile.centroid_embedding = weighted mean of accepted evidence
profile.radius = spread across accepted evidence
profile.exemplars = best clean examples
profile.sample_count = accepted evidence count
```

If any gate fails, do not update and surface the reason to the user ("cluster too noisy — std_dev 0.14", "audio too short", etc.) so they understand why automatic improvement didn't happen.

## Biometric data handling

Voice embeddings are sensitive local biometric data. Store transcript chunk embeddings, transcript speaker centroids, and global profile embeddings encrypted at rest. Keep raw vectors out of logs. Users must be able to delete a profile, delete all voice data, or keep a transcript while removing its embeddings.

## Notes

- Depends on stories 0059, 0060, 0061
- The update should be opt-in (prompt the user) unless they have previously opted into automatic improvement
- The `Other` label should never update a global profile — only named enrolled speakers
- Store a `last_session_update` timestamp on the profile so the user can see when it was last improved from transcript evidence
- Manual enrollment can remain as an optional way to seed a profile, but transcript-confirmed evidence should be the main learning path

## Completion note (2026-07-09)

Implemented in `src-tauri/src/services/voice_learning.rs`. Gates per story:
clean speech >= 6 s, duration-weighted purity >= 0.80, no clipped chunks in the
group, mean session score >= 0.85, score std dev <= 0.06, mean margin >= 0.15,
and target profile must beat the next closest profile by >= 0.05. Profiles are
rebuildable: `VoiceprintProfile` stores `enrollment_embedding` plus
`evidence: Vec<ProfileEvidence>` and the global embedding is recomputed from
those parts (evidence keyed by note+speaker; re-accepting replaces, never
duplicates). All new vectors seal/unseal through `VoiceEmbeddingStore`.
Learning is gated by `voice_learning_enabled` and, when
`voice_embeddings_encryption_required`, by the store actually being encrypted.
IPC: `voiceprint_evaluate_session_evidence` / `voiceprint_apply_session_evidence`.
UI: after a correction to an enrolled profile, TranscriptPanel offers
"Improve <name>'s voiceprint from this recording?" only when the gates pass.
