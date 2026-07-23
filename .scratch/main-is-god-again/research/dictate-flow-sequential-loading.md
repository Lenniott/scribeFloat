# Research: sequential-loading habits in the live Dictate flow

**Date:** 2026-07-23. Traced read-only, ticket 27. Frontend note: `src/lib/stores/scribeController.svelte.ts` has no Dictate references — the whole round trip lives in `src-tauri/src/controllers/dictate.rs`.

## What happens

1. Key listener → `dispatch_action` → `spawn_dictate_window_and_start_inner_async` (`dictate.rs:503-511`): main-thread hop captures frontmost PID + opens HUD, then a hardcoded `sleep(50ms)` before checking the result and calling `start()`. Same blind-sleep pattern in `spawn_dictate_hold_while_held` (`dictate.rs:459`) and `spawn_dictate_hold_immediate_stop` (`dictate.rs:492`).
2. `start()` (`dictate.rs:548`) resolves the input device and opens the CoreAudio stream (`audio.rs:208-238`) — real ordering vs. the HUD, but device *enumeration* has no dependency on step 1's paste-target capture; it's just always called from inside the same chain.
3. Only after `start()` sets `Recording` does it call `spawn_record_start_preload()` (`dictate.rs:595, 603-614`) — the first point Whisper preload begins. Confirms the "wait until the last possible moment" pattern already flagged for onboarding also holds in live Dictate: preload isn't kicked off at key-down, HUD-open, or device-resolve — only after the mic is already live.
4. Stop → `stop_and_transcribe` (`dictate.rs:733`) emits `Transcribing`, spawns `do_transcription` in `spawn_blocking`.
5. `do_transcription` (`dictate.rs:802`): `stop_and_finalize()` → `read_wav_mono_f32` → length check → `model_available()` check → `run_post_capture_transcription` (inference, out of scope) → format text → history append (`dictate.rs:931`) → clipboard write (`dictate.rs:940`) → paste on main thread (`dictate.rs:951-975`) → delete temp wav → `sleep(DICTATE_COMPLETE_HOLD)` → idle.

## Dependency read

- Steps 1→2→3 are a real main-thread hop chain, but the 50ms sleeps are blind waits polling for `run_on_main_thread` completion instead of awaiting the callback result directly.
- Whisper preload (tail of `start()`) depends only on config (`model.default_model_path()`), not on mic/audio state — it could start at key-down or HUD-open time, not after `Recording` is set.
- WAV read → transcription → text format is a genuine dependency chain.
- History append and clipboard write both depend only on `text`, not on each other — currently sequential for no reason. Clipboard legitimately gates paste; history append doesn't need to precede clipboard/paste but sits on that critical path today.
- Temp-wav deletion doesn't depend on paste outcome and could run right after `pcm_16k` is read, instead of waiting until after paste.

## The habit, named

Same habit as startup: independent-but-adjacent steps are written as one sequential chain, and "start the expensive background thing" (Whisper preload) sits at the *end* of mic setup instead of the earliest idle point — the same pattern already known from onboarding's cold-load issue, recurring in the main app's own Dictate path.

## Candidate follow-on tickets

- Start Whisper preload at key-down/HUD-request time, not after `Recording` is set in `start()` — removes cold-load wait from the critical path for short recordings.
- Replace the fixed `sleep(50ms)` main-thread-hop waits (`dictate.rs:459, 493, 505`) with a real completion signal from `run_on_main_thread` instead of blind sleeps.
- Run clipboard write and history append concurrently (or move history append after paste) since neither depends on the other and only clipboard gates paste.
- Delete the temp WAV right after `pcm_16k` is read successfully, instead of after paste, since nothing downstream re-reads the file.
