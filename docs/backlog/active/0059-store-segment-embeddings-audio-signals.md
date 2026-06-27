---
id: "0059"
title: Store per-segment embeddings and audio quality signals at label time
status: active
---

# Store per-segment embeddings and audio quality signals at label time

Currently `label_segments()` in `voiceprint.rs` computes a 192-float embedding for each Whisper segment and immediately discards it after the cosine comparison. The label is stored on `SpeakerBlock` but nothing else is kept.

Extend `SpeakerBlock` to carry the embedding and three audio quality signals computed from the raw PCM slice before embedding:

- `embedding: Option<Vec<f32>>` — the L2-normalised 192-float vector
- `audio_duration_s: f32` — length of the PCM slice in seconds
- `vad_purity: f32` — fraction of frames above the RMS noise floor (speech vs silence)
- `rms_energy: f32` — mean RMS of the slice
- `clipping: bool` — true if any sample is at or beyond ±1.0

These fields feed the session-centroid and quality-gate stories (0060, 0061, 0063). Without storing the embedding here, all downstream stories require re-embedding from stored PCM.

## Notes

- `SpeakerBlock` is serialised to the session JSON; the embedding adds ~3 KB per segment — acceptable at typical meeting lengths
- Segments that were too short to embed (< 2 s) should have `embedding: None`; the audio signals should still be populated so we know why the embed was skipped
- Do not change the labeling logic itself in this story — just persist what is already computed
