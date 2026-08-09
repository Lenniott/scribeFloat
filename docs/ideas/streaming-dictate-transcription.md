# Streaming / chunked Dictate transcription

Parked from [main-is-god-known-issues](../../.scratch/main-is-god-known-issues/MAP.md) ticket 14. Experiment with a real accuracy-vs-latency tradeoff — own wayfinder / branch.

## Summary
- Difficulty: **large / architectural**.
- Today: mic WAV streams to disk during capture, but Whisper runs only after Stop as one pass over the full buffer — every Dictate pays a fixed post-Stop latency tax.
- Idea: detect silence/VAD gaps live, transcribe finished chunks while recording continues, stitch at the end so Stop only waits on the tail.

## Why it's parked here
Not a localized fix. Needs live VAD during Recording, chunk lifecycle + partial transcription under the existing inference gate, stitching across chunk boundaries, and Dictate state-machine progress semantics. Chunk-boundary accuracy is an open product call before implementation.

## Research already done
Grounded findings live in the closed triage ticket:
`.scratch/main-is-god-known-issues/issues/z_14-streaming-transcription-before-stop.md`

Key anchors:
- Stop path: `DictateController::stop_and_transcribe` → `do_transcription` → `run_post_capture_transcription` (single Whisper pass)
- Capture already streams WAV; bottleneck is transcription-after-Stop only
- Existing `offline_cuts` / speaker-change cuts are post-hoc import path, not live silence chunking

## Suggested future destination
1. Decide accuracy policy for mid-sentence chunk boundaries (prompt carryover? overlap? accept some glue errors?).
2. Prototype VAD/silence chunking on Dictate only, measure latency win vs WER.
3. Keep Record path unchanged until Dictate proves out.
