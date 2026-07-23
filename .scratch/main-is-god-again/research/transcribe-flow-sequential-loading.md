# Research: sequential-loading habits in the Transcribe (Upload) flow

**Date:** 2026-07-23. Traced directly (read-only), ticket 29 under the map at `.scratch/main-is-god-again/MAP.md`. Model *load time* (Whisper/Sortformer/VAD inference latency) is explicitly out of scope; this is about orchestration/sequencing only.

## What happens

`TranscribeController::start` (`controllers/transcribe.rs:86-169`) does cheap sync validation, flips state, then spawns the whole batch onto `spawn_blocking` (good — doesn't block the async runtime). Inside `run_batch` (`transcribe.rs:192-382`), items are processed in a plain `for` loop, one at a time, each going through the full stage sequence before the next item starts:

1. `self.input.decode_input(input)` (`transcribe.rs:222`, `services/transcribe_input.rs:105-127`) — ffmpeg-style decode + linear resample to 16kHz, CPU/file-I/O bound. Independent per item.
2. `offline_cuts(&decoded.mic_pcm_16k)` (`transcribe.rs:239`) — pitch analysis, depends only on step 1's output for *this* item.
3. `run_post_capture_transcription` → `transcribe_pcm_with_progress` (`services/model.rs:417-450`): re-checks Whisper integrity via **synchronous SHA-256 hash of the model file** (`model.rs:436-444`) *after* decode has produced PCM, immediately before inference. Doesn't depend on the decoded audio — a fixed per-launch cost gated onto every item's critical path instead of validated once.
4. ASR pass(es) — the actual Whisper inference (out of scope for speed, in scope for placement).
5. Diarization, Upload profile only (`services/transcription.rs:157-162, 269-286`): `SpeakerEvidenceInput::DiarizeOnDemand` calls `Diarizer::diarize`, which does `self.load_sortformer()` (`services/diarization.rs:262-266`) — a full model load from disk — **after** ASR has already produced segments, even though loading the Sortformer file has zero dependency on ASR's output. Only the later `align_ranges_to_segments` step (`transcription.rs:295`) genuinely needs the ASR segments.
6. `.md` transcript write (`transcribe.rs:305-343`, `services/output.rs`) — depends on segments/speaker_blocks from steps 4-5.
7. `HistoryService::append` (`transcribe.rs:361-365`) — depends on the same, plus decides the markdown path.
8. `app.emit("transcribe://state-changed"/"note://item-added")` — UI update, depends on above.
9. Loop continues to item N+1's decode (`transcribe.rs:206`) — only *after* item N's history write and UI emit are fully done.

## Habit, named

Two variants of the startup habit: (a) a fixed-cost, input-independent check (Whisper SHA-256) reruns synchronously on every item's critical path instead of once; (b) an independent model load (Sortformer) sits strictly *after* the stage whose output it doesn't need, instead of starting as soon as decoded PCM exists — its multi-hundred-MB read waits behind ASR instead of overlapping it. The queue loop is also a single `for`: item N+1's decode (pure file I/O, no dependency on item N) can't start until item N finishes writing and journaling.

## Candidate follow-on tickets

- Load the Sortformer model concurrently with (or ahead of) the ASR pass for Upload items, since diarization only needs decoded PCM + segments at alignment time, not at load time — not the model itself, just when its file read starts.
- Stop re-hashing the Whisper model synchronously per transcription call inside `transcribe_pcm_with_progress`; cache/validate once per run (mirrors the VAD-hash fix already proposed for startup, ticket 25).
- Prefetch/decode the next queued item's audio while the current item's ASR+diarization is in flight, rather than gating decode on the previous item's full write+journal+emit cycle.
