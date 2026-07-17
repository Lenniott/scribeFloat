# Speaker Detection Replacement — Final Outcome (was: Parakeet migration plan)

> **Status: COMPLETE (2026-07-17), with a materially different scope than the
> original plan below described.** This document is kept as the audit record of
> what was planned, what was measured, and what was actually shipped.
> The durable decision record is [ADR-0014](../adr/0014-anonymous-diarization-replaces-voiceprint-identity.md).

## What actually shipped

The original plan proposed replacing Whisper + Silero VAD with a full Parakeet
TDT + Sortformer pipeline. A benchmark spike (2026-07-16, Apple M1 16 GB, using
`tests/short_mic.wav` and the 93-minute `tests/mic.wav`) changed the decision:

| Engine | Speed | Peak RSS | Notes |
|---|---|---|---|
| Whisper small.en (Metal, prod) | 19–21× realtime | 0.4–1.3 GB | near-idle CPU (GPU does the work) |
| Parakeet TDT v3 fp32 (CPU) | 8.7–9.3× | 2.7–4.6 GB | 2.55 GB on disk, ~3 busy cores |
| Parakeet TDT v3 int8 (CPU) | 14–29× | 1.4–2.7 GB | 670 MB on disk |
| Sortformer v2 diarization | 18–23× | 0.9–3.5 GB full-pass | found 2–3 speakers correctly; **streaming ≈ 0.2 cores live** |

Conclusions the user ratified:

1. **Whisper stays.** Its accuracy is sufficient, it runs on Metal with near-idle
   CPU, and swapping ASR bought nothing the product needed. Parakeet TDT is
   **not** adopted (if ever revisited: int8 beat fp32 decisively, and ~2-minute
   ASR chunks beat 4-minute on both speed and RAM).
2. **Sortformer is adopted — live.** Because Sortformer is streaming-native, it
   runs *during* recording fed by the mic writer-thread PCM tap; at stop, only
   Whisper remains. Upload runs one post-capture full pass. Dictate and
   dual-source Record do not diarize.
3. **The real goal was privacy, not a model swap.** The voiceprint engine
   (ADR-0011) and every stored voice embedding were deleted; a one-time startup
   purge migrates profile names into a plain `speaker_names.json` store and
   removes `voiceprints/`, `voiceprint_clips/`, and the keychain key.

## Shipped architecture (see ADR-0014 for rationale)

- `src-tauri/src/services/diarization.rs` — owns Sortformer
  (`diar_streaming_sortformer_4spk-v2.onnx`, bundled, sha256-pinned).
  `Diarizer` (full pass) + `StreamingDiarizer` traits; `LiveDiarization` worker
  spawned per Record, fed via the tap, flushed after `stop_and_finalize()`.
- `src-tauri/src/services/speaker_align.rs` — pure max-overlap alignment of ASR
  segments to `DiarizationRange`s → `Speaker 1..4` blocks, `Other` for
  un-diarized speech, adjacent same-label merge.
- `src-tauri/src/services/transcription.rs` — seam takes
  `SpeakerEvidenceInput::LiveRanges` (Record) or `::DiarizeOnDemand` (Upload);
  any diarization failure degrades to a plain transcript.
- `src-tauri/src/services/speaker_names.rs` — plain name store;
  `note_relabel_speaker` renames every matching block in a note and saves the
  name globally (auto-assigned labels excepted).
- `src-tauri/src/services/legacy_voice_purge.rs` — idempotent startup purge.
- Frontend: TranscriptPanel relabel picker (saved names + free text), Settings →
  Voice as name management, onboarding enrollment step removed, advanced voice
  settings removed.

## Verification shipped with it

Rust: alignment, worker-loop, seam-behavior (fake inference + fake diarizer),
name-store, relabel, purge, and legacy-deserialization tests (old history lines
with embeddings still load; rewritten lines contain none). Frontend: Vitest
suites for TranscriptPanel relabeling, name management, onboarding without
enrollment, advanced settings without voice controls.

Manual smoke checklist (run with the real model in the app models dir):

1. Record 30–60 s with two speakers → note shows `Speaker 1/2` blocks; click a
   label → rename to a real name → all matching blocks update; the name appears
   in Settings → Voices and in the next note's picker.
2. Upload a multi-speaker file → labeled blocks. Dictate → plain pasted text.
   Dual-source Record → `In`/`Out` labels.
3. Remove the Sortformer model file → recording still produces a plain
   transcript (warn in logs, no crash).
4. Launch against pre-upgrade app data → profile names appear as plain names,
   `voiceprints/` and `voiceprint_clips/` are gone, old notes still render, and
   `history.jsonl` contains no `"embedding"` after compaction.

---

*The remainder of the original Parakeet migration plan was deleted; see git
history (`docs/audits/PLAN_Model_replace.md` before 2026-07-17) for the full
proposal, and the benchmark session results for the measurements that
overturned it.*
