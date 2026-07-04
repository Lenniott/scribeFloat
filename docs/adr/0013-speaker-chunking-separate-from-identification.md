# ADR-0013: Speaker-aware chunking is separate from speaker identification

## Status

Accepted

## Context

Whisper segments are cut internally at silence. In real conversation, speakers often hand over with no measurable pause, so one segment can contain two voices. That hurts transcript quality and makes downstream speaker labelling harder.

Two problems are often conflated:

| Problem | Question | Example signal |
|---------|----------|----------------|
| **Turn boundaries** | *Where* did the voice likely change? | Pitch jump, loudness jump, silence gap |
| **Speaker identity** | *Who* is speaking? | Voiceprint embedding vs enrolled profiles |

Pitch and speaker embeddings solve different problems. Pitch measures *how* the voice sounds over time (fundamental frequency, level). A Hz jump might mean a new person **or** the same person emphasising a word — deciding which is the hard part, and pitch alone cannot do it reliably. Speaker embeddings (see ADR-0011) answer *who* by comparing timbre to enrolled profiles.

Neural diarization (pyannote, sherpa diarization) was evaluated on the pitch_test benchmark. At best tuning it matched light chunking recall (4/5) on the same fixture while adding model download, latency, and a different failure mode. It does not replace turn-boundary detection for our use case; its strengths belong in identification/clustering, not in deciding where to cut before Whisper.

ADR-0011 assumed Whisper segments were the right granularity for voiceprint embedding. That holds when segments are single-speaker; when they are not, embeddings are computed on mixed audio and labels degrade.

## Decision

We will add a **pre-Whisper chunking layer**, gated by `Config.speaker_aware_chunking` and defaulting off until the Rust port matches the benchmark, that finds likely speaker handovers and runs Whisper per chunk with timestamp offsets merged back.

**Chunk boundary signals (union, bias toward over-cutting):**

- Pitch jumps from a Rust pitch tracker. The intended implementation is still a parity-checked `pyin` port of the Python prototype; the current inline YIN implementation is experimental and did not meet the benchmark target.
- Loudness jumps via in-repo RMS (`pcm_rms`)
- Silence gaps

**Analysis details:**

- Cuts are logged (`tracing::info!`) and stored on `SessionManifest.speaker_cuts`
- Analysis runs on the 16 kHz PCM used by Whisper. Before this is enabled by default, the pitch track must be parity-checked against the Python prototype at the same sample rate.
- Voiceprint / `speaker_blocks.rs` is unchanged — it still identifies *who* on the resulting segments

**Scope (v1):** Scribe mic path and file-upload transcribe mic path only. Not Dictate, not speaker loopback.

**Rejected for this stage:**

| Option | Reason |
|--------|--------|
| sherpa-onnx diarization | New model download; no recall advantage on benchmark |
| Fixed-duration PCM windows | Regression source; whisper.cpp already window-seeks |
| Pitch as speaker ID | Cannot distinguish new speaker from same-speaker emphasis |

## Consequences

**Easier:**

- Segments are less likely to span two speakers before voiceprint runs
- Loudness/silence use existing math; the pitch tracker remains the main validation risk
- Extra cuts are cheap; missed handovers are expensive — over-cutting is acceptable

**Harder:**

- Multi-pass Whisper when cuts fire (more encode time than single pass)
- pYIN analysis adds ~tens of seconds on long recordings in release builds
- Two voices within ~2 semitones and similar level remain indistinguishable at this stage (benchmark: 1/5 handovers) — voiceprint owns that case
- ADR-0011 consequence "no new audio chunking layer" is amended: outer chunking may run for Scribe mic and upload paths when explicitly enabled

**Out of scope:**

- Full multi-speaker diarization (labelling every unknown speaker)
- Replacing per-session centroids or mid-session voiceprint capture (see backlog 0055, 0060)
- Real-time cut detection during live capture

## References

- Shipped: story 0070 (`docs/backlog/archived/0070-speaker-aware-chunking.md`)
- Prototype direction: `pitch_test/DIRECTION.md`
- Voiceprint: ADR-0011
