# ADR-0013: Live pitch analysis with cuts in HistoryRecord, timeline in analysis.json

**Status:** Binding
**Wayfinder:** Implemented — Main is God again / current product (`pitch-detection`, `analysis.json`).

## Context

We want lightweight voice-change detection (pitch jumps, loudness jumps) computed live
while a Record session streams to `mic.wav`, to enable smarter Whisper chunking and
change-cut hints. This is NOT speaker identity — see [ADR-0014](0014-anonymous-diarization-replaces-voiceprint-identity.md) for current speaker labelling; ADR-0011 is superseded history only. Three decisions were forced:

1. **Which pitch crate.** The `pitch` crate (bitstream autocorrelation) hardcodes a
   48 kHz sample rate (`const SPS: u32 = 48_000`), takes `f64` slices, and returns peak
   amplitude rather than RMS — every value would need rescaling for our 16 kHz stream.
   `pitch-detection` (McLeodDetector) takes the true sample rate directly and its
   power/clarity thresholds provide a principled voicing gate. An offline prototype
   using McLeod caught 4/5 real speaker changes (avg error 0.31 s, 9 extra cuts) in
   union mode; consensus mode (≥2 signals) caught only 1/5.

2. **Where analysis runs.** The cpal callback is real-time and must never block. The
   WAV writer thread already tolerates ~100 ms latency and sees the post-resample
   16 kHz samples — the exact samples written to disk, so analysis timestamps equal
   mic.wav/Whisper timestamps with no offset bookkeeping.

3. **Where results live.** `session.json` is deleted after successful transcription.
   The full frame timeline is ~56 k frames (~0.5–1 MB JSON) per recorded hour — far too
   large for `history.jsonl`, which is parsed on every history load. Detected cuts are
   typically < 100 entries.

## Decision

- Pitch detection uses the `pitch-detection` crate (McLeod), not `pitch`.
- Analysis runs on the WAV writer thread via an optional `Pcm16kTap` observer on
  `AudioService::start_mic`; `services/analysis.rs` is a pure module (no I/O, no locks)
  owning the streaming `PitchAnalyzer` and `detect_cuts`.
- Default cut configuration is the benchmarked one: union mode (pitch OR loudness),
  silence cuts off, consensus off. Silence/consensus remain config options only.
- Storage split: **cuts** go on `SessionManifest` (crash-recovery window, from the
  Transcribing write onward) and durably on `HistoryRecord.speaker_change_cuts`; the
  **full frame timeline** goes to `{session_dir}/analysis.json`, which survives
  `keep_wav = true` and is deleted with the session dir otherwise.
- Analysis never fails a save — harvest errors are logged and the save proceeds.

## Consequences

- Change-cut data is available the moment recording stops, with no second pass over
  the audio; future Whisper chunking can consume `PreparedAudio.speaker_change_cuts`.
- `history.jsonl` stays small; anything needing the full pitch/loudness timeline must
  read `analysis.json` and therefore only works when audio was kept.
- The McLeodDetector holds `Rc<RefCell<..>>` internals and is not `Send`, so it is
  constructed per voiced window (~15.6/s) instead of being owned by the analyzer that
  crosses threads. Measured cost is negligible; do not "optimize" it back into a field.
- A cut says "the voice changed here" — spans between cuts must not be presented as
  speaker identities. Speaker labels come from anonymous diarization ([ADR-0014](0014-anonymous-diarization-replaces-voiceprint-identity.md)), not from pitch cuts or voiceprint identity (superseded [ADR-0011](0011-voiceprint-engine-binary-speaker-verification.md)).
- MFCC fingerprinting / speaker clustering is explicitly out of scope.
