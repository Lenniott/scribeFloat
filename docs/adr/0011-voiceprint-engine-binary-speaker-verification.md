# ADR-0011: Voiceprint Engine — Binary Speaker Verification over Full Diarization

## Status

Superseded by [ADR-0014](0014-anonymous-diarization-replaces-voiceprint-identity.md) — the voiceprint engine and all stored biometric data were removed; anonymous Sortformer diarization with plain renameable speaker names replaced it.

## Context

ScribeFloat captures audio from a single microphone. When the user records a meeting or conversation, the resulting transcript mixes their voice with other speakers — making it hard to extract just the user's own thinking, decisions, and knowledge contributions.

The obvious solution is speaker diarization (who said what), but full diarization is the wrong fit here:

- The user's knowledge is already the product's focus; it's the *external* voice that refines the user's position, not the reverse.
- Knowing whether a segment belongs to the user is sufficient; labelling every third-party speaker is not required.
- Full diarization pipelines (pyannote, spectral clustering) are heavy, latency-sensitive, and require a different audio chunking strategy.

Four Rust crates were evaluated for speaker identity in the pipeline:

| Crate | Verdict | Reason rejected |
|-------|---------|-----------------|
| `polyvoice` | Rejected | Thin wrapper, no prebuilt binaries, unclear model support |
| `parakeet-rs` | Rejected | Hard 4–5 minute audio limit (architectural, not configurable) |
| `pyannote-rs` | Rejected | Abandoned (last commit Sep 2025); 30-second audio limit (issue #8, unresolved, zero maintainer response) |
| `speakrs` | Viable but deferred | Depends on `ort 2.0.0-rc.12` pre-release; ort 1.x yanked; RC dependency is a production risk |
| `sherpa-onnx` | **Selected** | Official k2-fsa crate; ships prebuilt ONNX runtime; no ort dependency; v1.13.3; 50k downloads; active maintenance |

Empirical validation with the `tools/voice-probe` harness confirmed that the campplus model cleanly separates the user's voice from other speakers at cosine similarity 0.75:

- ME range tested: 0.841–0.933 across built-in mic close, built-in mic far, phone audio
- OTHER max observed: 0.455 (same-gender, phone audio — the hardest case)
- Gap (min-ME minus max-OTHER): +0.386 at worst
- F1 = 1.0 (no misses, no leaks) from threshold 0.35 through 0.90

The binary model ("is this the user?") naturally extends to N named profiles ("is this Alice? is this Bob?") by nearest-neighbour lookup against all enrolled profiles with a shared threshold, falling back to "Other" when no profile exceeds the threshold.

## Decision

We will implement a **binary-extensible speaker verification engine** using `sherpa-onnx` with the 3D-Speaker campplus ONNX model.

**Model:** `3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx` (~25 MB, downloaded locally, never sent to a server). Input: 16 kHz mono PCM. Output: 256-dimensional L2-normalised speaker embedding.

**Threshold:** 0.75 cosine similarity (default). User-adjustable via `voice_similarity_threshold` config field.

**Identify algorithm:** Nearest-neighbour across all enrolled profiles; return the profile name if `max_sim >= threshold`, else return `"Other"`.

**Embedding units:** Whisper segments. Each segment already has `[start_ms, end_ms]`; the PCM slice is extracted from the session buffer, resampled to 16 kHz mono if needed, and fed to the embedding extractor. No additional chunking layer.

**Enrollment sources:**
- **First-time onboarding** — guided clip capture on first launch; same flow for every profile (user or others).
- **Mid-session capture** — a capture button in Record mode lets the user grab a voiceprint while another person is speaking; VAD purity gating ensures quality (minimum 5 s of clean speech, 10 s preferred).
- **Settings → Voice** — add more prints to any existing profile at any time to improve accuracy across distances and microphones.

The enrollment UX is identical for all profiles. The name field is always an auto-fill: pick an existing profile (to add another clip to it) or type a new name. "You" and "Alice" enroll via the same flow.

**Profile store:** JSON file per user at `~/.scribefloat/voiceprints/<profile-name>.json`, keyed by mic device ID so distance/mic variation is handled automatically.

**Transcript output:** Consecutive same-label Whisper segments are merged into `SpeakerBlock { label, start_ms, end_ms, text }`. Rendered as:

```
[You]   [00:00 → 02:00]  text...
[Other] [02:00 → 03:00]  text...
```

The user's display label defaults to `"You"` and is configurable via `user_display_name`.

## Consequences

**Easier:**
- No new audio chunking layer — Whisper segments are already the right granularity.
- Binary verification is dramatically simpler than full diarization (no clustering, no speaker count estimation).
- Zero network dependency — all inference runs locally on the user's machine.
- Naturally extensible to N named stakeholder profiles without changing the core identify algorithm.
- Unified enrollment UX for all profiles (user and others) reduces cognitive overhead.
- The threshold is empirically calibrated via `tools/voice-probe` and is user-adjustable.

**Harder:**
- `sherpa-onnx` adds a ~25 MB ONNX model file to the user's data directory (downloaded on first use, not bundled).
- Enrollment quality degrades in noisy environments; VAD purity gating mitigates but doesn't eliminate this.
- Short Whisper segments (< 2 s of speech) may not produce reliable embeddings; segments below the minimum are labelled "Other" conservatively.
- Profile accuracy on laptop mic varies with distance; multi-distance enrollment is recommended during onboarding.

**Out of scope:**
- Full multi-speaker diarization (who said what for every speaker) — this is a later phase if user research demands it.
- Cloud-based speaker verification — local-only is a product commitment.
- Real-time segment labelling during recording — labels are applied at transcription time, not during live capture.
