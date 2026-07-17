# ADR-0014: Anonymous Live Diarization Replaces Voiceprint Identity

## Status

Accepted — supersedes ADR-0011.

## Context

The voiceprint engine (ADR-0011) stored per-person voice embeddings on disk and
in history records to guess *who* was speaking. That approach had two problems
that grew rather than shrank:

1. **It stored biometric data.** Voice embeddings are biometric identifiers.
   Enrollment clips, per-note chunk embeddings, session centroids, profile
   evidence, and an at-rest encryption key all existed to support identity
   guessing — a heavy privacy liability for a note-taking app.
2. **The guessing was unreliable.** Similarity-scoring chunks against enrolled
   profiles produced wrong labels often enough that users were asked to correct
   them, and the correction/learning loop added still more stored biometrics.

What users actually need is "who spoke when" *within one recording*, with
labels they can rename. A benchmark (2026-07-16, Apple M1) showed NVIDIA
Sortformer v2 — a streaming diarization model available via `parakeet-rs` —
separates 2–4 speakers accurately at ~19× realtime, and its streaming API can
run *during* recording at ~0.2 CPU cores, so speaker evidence is finished the
moment recording stops. Whisper ASR quality was judged sufficient; a full ASR
swap (Parakeet TDT) was evaluated and explicitly rejected.

## Decision

Speaker identity is removed from the product. Diarization replaces it:

- **Sortformer v2** (`diar_streaming_sortformer_4spk-v2.onnx`, bundled, 492 MB)
  produces anonymous speaker time-ranges. `services/diarization.rs` owns the
  model; controllers never touch ONNX.
- **Live for Record**: a worker thread is fed by the existing mic writer-thread
  16 kHz PCM tap (the ADR-0013 mechanism) via `feed()`, flushed after
  `stop_and_finalize()` joins the writer. Upload runs one full-audio pass
  post-capture. Dictate and dual-source Record do not diarize (dual-source
  keeps `In`/`Out` channel labels).
- **Alignment**: ASR segments take the label of the diarization speaker with
  maximum summed time-overlap (`services/speaker_align.rs`); no overlap →
  `Other`. Labels are anonymous slots `Speaker 1`–`Speaker 4` (the model's
  4-speaker cap; >4 people merge into the nearest slot).
- **Names are plain text**: `speaker_names.json` stores `{name, slug,
  created_at, updated_at}`. Relabeling a speaker in a note
  (`note_relabel_speaker`) rewrites matching block labels and saves the name
  globally for reuse. No audio, no vectors, nothing to enroll.
- **Degrade policy**: any diarization failure (missing model, worker error)
  produces a plain unlabeled transcript. Diarization never fails a note and is
  never retried with a full pass for Record.
- **Biometric purge**: on first launch after upgrade, voiceprint profile names
  are imported into the name store, then `voiceprints/`, `voiceprint_clips/`,
  and the keychain encryption key are deleted. Embedding fields no longer exist
  on record types, so history compaction rewrites `history.jsonl` without them.
  Old notes keep transcripts, labels, and correction badges.

## Consequences

- **Easier**: no biometric storage, encryption, retention settings, enrollment
  UI, or learning gates — five backend services, ~28 IPC commands, and three
  crates (sherpa-onnx, ring, base64) deleted. Privacy posture is structural:
  the diarization model has no concept of identity.
- **Easier**: speaker labels exist the moment transcription finishes; live
  collection costs ~0.2 cores during recording and adds <1 s at stop.
- **Harder / accepted**: speakers are anonymous per note — "Speaker 1" in two
  different notes is not the same person until a human renames both. Cross-note
  identity is off the table by design.
- **Accepted**: at most 4 speaker slots per recording; rapid exchanges inside
  one coarse Whisper segment are attributed to the max-overlap speaker.
- **Accepted**: relabeling matches by label string; legacy notes where two
  distinct speakers shared a label merge under the new name. The old
  chunk-cascade correction machinery is gone.
- ADR-0011 is superseded; its `[You]`/`[Other]` binary-verification model and
  the 0.75 similarity threshold no longer exist. ADR-0013's pitch/loudness
  cuts remain as identity-free timeline enrichment.
