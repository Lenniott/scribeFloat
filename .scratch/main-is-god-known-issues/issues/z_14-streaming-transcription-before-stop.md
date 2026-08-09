---
title: "Triage: Start transcript work before Dictate fully stops"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Resolution

**Later, closed 2026-08-09.** Parked as an experiment to [`docs/ideas/streaming-dictate-transcription.md`](../../../docs/ideas/streaming-dictate-transcription.md). Needs live VAD, chunk-and-stitch pipeline, state-machine changes, and an accuracy-vs-latency product call — own wayfinder/branch.

## Issue

Dictate transcription only starts after the user releases/stops recording, then runs as a single Whisper pass over the entire buffer — so every Dictate session pays a fixed latency tax after Stop, proportional to how long the recording was. Raw mic audio is already streamed to disk during capture; only transcription itself is not incremental. No live silence/VAD detection exists to chunk work earlier.

## Question

Read the "Start transcript work before Dictate fully stops" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Later** — large and architecturally significant: needs new live VAD/silence detection during capture, a chunk-and-stitch transcription pipeline, Dictate state-machine changes, and a real accuracy-vs-latency tradeoff decision (chunk-boundary errors) before implementation. Confirm this stays its own future wayfinder rather than folding into this effort?

## Findings

**Current stop→transcribe flow (Dictate), confirmed single-shot, everything after Stop:**

- `src-tauri/src/controllers/dictate.rs:739` (`stop_and_transcribe`) — on stop, flips state `Recording → Transcribing`, emits a `PreparingAudio` progress event, takes the session, and dispatches the entire pipeline onto a `tokio::task::spawn_blocking` (`do_transcription`, line ~776-804). Nothing has been transcribed yet at this point.
- `do_transcription` (`dictate.rs:808-899+`):
  1. `session.mic.stop_and_finalize()` (line 816) — finalizes/closes the WAV file that was being streamed to disk during capture.
  2. `read_wav_mono_f32(&wav_path)` (line 817) — reads the **entire** finalized WAV back into memory as `pcm_16k`.
  3. Minimum-length check (100ms).
  4. `run_post_capture_transcription(...)` (line 858) with `CaptureProfile::Dictate`, `speaker_pcm_16k: None` — this is a **single Whisper pass over the full PCM buffer** (see `transcription.rs:241-301`, the `else` branch since Dictate is never dual-source). No chunking, no silence-based segmentation of work already done.
  5. Result formatted via `format_dictate_segments`, then paste/clipboard flow.
- The mic audio itself IS streamed to disk incrementally during recording (`AudioService`/`MicSession`, `mic.wav` written live) — so raw capture is already non-blocking. The bottleneck is that **transcription only starts after Stop**, and runs over the whole buffer at once.
- No existing silence/VAD-based chunk-boundary detection was found in `dictate.rs` or `transcription.rs` for the Dictate path. (`transcribe.rs:239` has an `offline_cuts`/`speaker_change_cuts` mechanism, but that's for the Transcribe/import flow's speaker-change segmentation, not silence-triggered ASR chunking, and it runs post-hoc over the full buffer too — not incrementally during capture.)

**What a chunked/streaming approach would need to change:**
1. **Live silence/VAD detection during Recording** — nothing currently inspects the live mic PCM stream for silence gaps while `DictateState::Recording` is active; this is new capability, likely hooking into the same PCM tap used for diarization evidence collection (`SpeakerEvidenceInput`/live capture path) or the audio-level meter callback already in `dictate.rs` (`DICTATE_AUDIO_LEVEL_EVENT`).
2. **Chunk boundary + partial-buffer transcription** — need to slice the growing WAV/PCM into finalized chunks, run `run_post_capture_transcription` (or a lower-level pass) per chunk while recording continues, likely on a background task that doesn't fight the eventual final `do_transcription` call for the same model/inference lock (`inference_gate` mentioned in transcription.rs comments — Whisper passes are already serialized there).
3. **Stitching** — merging per-chunk `Segment` lists into one final transcript in order, handling a chunk boundary that lands mid-sentence (accuracy risk explicitly flagged in the source note) and reconciling timestamps across chunks.
4. **State machine changes** — `DictateState`/`DictateStateEvent` and the stop flow would need to know "N chunks already done, transcribe only the tail," changing the meaning of the `Transcribing` progress stage.
5. **Whisper context loss at chunk boundaries** — Whisper transcribes each chunk independently with no cross-chunk context, which can hurt accuracy at boundaries (e.g. word cut mid-sentence, no preceding-context prompt) — an open unknown the source note also flags.

**Size estimate: Large / architecturally significant.** This is not a localized fix — it requires new live-audio analysis (VAD/silence detection) during capture, a new chunk-lifecycle and partial-transcription-then-stitch pipeline, and changes to the Dictate state machine's progress semantics. It also has a real accuracy/UX tradeoff (chunk-boundary errors vs perceived latency win) that needs a design decision before implementation, not just an engineering task. Recommend treating as its own future wayfinder rather than in-scope for this triage effort, consistent with the original note's own "Later" framing.
