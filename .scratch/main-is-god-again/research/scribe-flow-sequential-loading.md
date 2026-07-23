# Research: sequential-loading habits in the Scribe (Record) flow

**Date:** 2026-07-23. Traced read-only, ticket 28.

## What happens

Traced `src-tauri/src/controllers/scribe.rs` (record start → stop → transcribe → diarize → merge → history → UI).

1. `ScribeController::start()` (`scribe.rs:156-291`): starts mic capture and, when speaker capture is on, loopback capture. Critically, `diarization.start_live_session()` (`scribe.rs:186`) starts **before and independently of** the mic/loopback streams, tapping the mic PCM via `diar_tap` (`scribe.rs:187,196-203`) as audio streams in — diarization already runs concurrently with capture, not after. `spawn_record_start_preload()` (`scribe.rs:289,421-430`) also fires a background `spawn_blocking` to preload the Whisper model context during recording, so the model is warm by the time the user stops (comment at `scribe.rs:417-421` documents this intent explicitly). **Both of these are already the correct pattern.**
2. `stop_and_save()` (`scribe.rs:561-674`): flips state to Transcribing, then calls `prepare_audio()` synchronously before handing off to a background task.
3. `prepare_audio()` (`scribe.rs:782-872`): finalizes mic WAV, then `live_diarization.finish()` (`scribe.rs:809`, comment at `805-808`: the channel must close first to avoid deadlock — a **genuine, correctly-ordered** dependency), then pitch/loudness harvest, then speaker-channel WAV merge (`scribe.rs:838-863`). Each step here depends on data unavailable until "stop" — legitimate sequencing, not a habit issue.
4. `do_transcription()` (`scribe.rs:677-776`, inside `spawn_blocking`) → `run_transcription()` (`scribe.rs:905-981`) → `run_post_capture_transcription` (`services/transcription.rs:134-205`).
5. Diarization ranges are already computed by this point (`SpeakerEvidenceInput::LiveRanges`, `scribe.rs:967-970`) — Whisper does **not** wait on diarization at all in the Record flow. Confirms diarization/transcription do not block each other here.
6. Inside `transcribe_capture_with_inference` (`transcription.rs:207-267`): when speaker capture produced a dual-source PCM, the code runs **two full Whisper passes strictly one after another on the same thread** — mic pass first (`transcription.rs:219-230`, 0-50% progress), then speaker pass (`transcription.rs:239-249`, 50-100%), then `merge_dual_source` (`transcription.rs:252`). These transcribe two independent, already-finalized PCM buffers; neither depends on the other's output, yet nothing backgrounds or parallelizes them.
7. `write_outputs()` (`scribe.rs:988-1120`): write markdown → `history.append` → emit `note://item-added` → cleanup session WAVs. Genuine dependency chain (transcript text needed before file write, etc.) — not a reordering candidate.
8. `Done` emitted (`scribe.rs:764-774`) only after history write completes — appropriate, UI needs `history_record_id`/`transcript_path`.

## The habit, named

Live diarization and model preload during capture are **already correctly backgrounded** — this flow does not exhibit the startup-style habit at that layer. The one place it does recur: two independent Whisper passes (mic track, speaker track) inside `transcribe_capture_with_inference` run back-to-back on one thread instead of concurrently — same "step A, wait, step B, but B doesn't depend on A" pattern as startup and Dictate, just showing up in the dual-source transcription pass instead of setup.

## Candidate follow-on tickets

- Run the mic-pass and speaker-pass Whisper transcriptions in `transcribe_capture_with_inference` (`services/transcription.rs:219-252`) concurrently (e.g. two blocking tasks joined before `merge_dual_source`) instead of sequentially — needs an audit of whether the shared inference/model context is safely reentrant from two threads at once; if not, may need a second lightweight decode context rather than true concurrency, but the sequencing itself is unjustified today.
- No changes needed for live diarization or model preload during capture (already concurrent, matches the good pattern).
- No changes needed for `write_outputs`' write→append→cleanup chain (genuine dependency chain).
