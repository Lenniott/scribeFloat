# Dictate replay results — 2026-09-19

The final implementation uses the bundled Whisper Small and unchanged decoding
settings. A shared background worker starts ASR on a confirmed pause after at
least 30 seconds of audio. Stop flushes the tail and returns ordinary segments.
The earlier paste-before-save fix remains in place.

| Replay | Existing warm ASR | Remaining ASR after Stop | Result |
|---|---:|---:|---|
| Original 15.000 s WAV | 1.123 s | 1.161 s | Same final text in all three live runs; 3.4% overhead |
| Synthetic 63.000 s sequence | 4.153 s | 2.135 s | 48.6% less waiting; wording differences remain |

Warm ASR is the median of baseline runs 2–3 (exclude the first inference).
Remaining ASR is the median of three short or two long **real-time** replays.
The long waits were 2.108 and 2.163 s. Timing excludes mic teardown, paste,
Note saving, model preload, and benchmark file decoding. These are ASR timings,
not a measured end-to-end Stop-to-paste promise.

## Audio and accuracy limits

`dicate-test.wav` is the user-provided mono int16, 44.1 kHz, 15-second recording.
SHA-256: `7548bda1064a9b21cee3d0af2e21aab2f0f0f425476ad55cd301af93be1539fd`.
The harness resamples it with the capture resampler and quantizes it exactly like
our WAV writer, so full-pass and live inference receive the same samples. The
long sequence is **four copies of that clip with one-second silent gaps**. It is
a controlled scheduling test, not a second independent speech recording.

The original clip's final text matches exactly across baseline/live runs. The
long recording splits at 31.744 s, leaving 31.256 s at Stop. Its raw ASR output
varies around “rapping/wrapping” and repeated words. Existing final-text cleanup
also collapses repeated passages differently depending on those variations.
Do not claim word-for-word equivalence or general WER improvement for long
dictations. There is no human-verified reference transcript or varied corpus.

The tests establish that segmentation loses or duplicates **no audio samples**;
that is a separate claim from whether Whisper recognizes every word correctly.
Broader natural-speech accuracy validation is a follow-up. An uninterrupted span
over 60 seconds or an overloaded queue falls back to the full WAV and may provide
no speedup. The 15-second case intentionally stays one job for context quality.

## Reproduce

From `src-tauri/`, with normal local GPU access:

```sh
cargo run --example dictate_benchmark -- ../.scratch/dictate-latency/dicate-test.wav ../.scratch/dictate-latency/baseline-and-live-short.json realtime 3
cargo run --example dictate_benchmark -- ../.scratch/dictate-latency/dicate-test.wav ../.scratch/dictate-latency/baseline-and-live-long.json realtime 2 4
```

The example never opens a microphone, writes Notes, or touches the clipboard.
Output JSON includes each timing, job boundary, raw segment list, and final text.
WAV and JSON artifacts are locally gitignored. Sandbox GPU allocation failed;
these measurements were run with normal GPU access. Avoid competing inference or
compilation during benchmarking; timings are machine-load dependent.

## Experiments not retained

- Four-second minimum and 256 ms pauses: short-clip wait ~1.10 → ~0.88 s,
  but changed “rapping” to “wrapping.”
- Bounded previous-text prompts: did not repair that change.
- Flash attention: somewhat faster, but changed recognition; not enabled.
- Skipping the per-job inference VAD: did not resolve the repeated-audio variation.
- Fifteen-second spans: larger speedup on synthetic long audio, but more spelling
  variation. Thirty seconds retains more acoustic context.

Slow Note saving (previously ~12–15 s) remains a separate issue. It no longer
blocks paste, but still delays the HUD returning to Idle.
