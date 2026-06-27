---
id: "0064"
title: Fix transcription progress bar to reflect where time is actually spent
status: active
---

# Fix transcription progress bar to reflect where time is actually spent

As a user waiting for a transcript, I want the progress bar to move in proportion to the work being done so that I know the app is doing something useful and can estimate how long it will take — not stare at a spinner for 80% of the time and then watch it race to 100%.

## The Problem

The current progress bar has its resolution backwards. The indeterminate `LoadingModel` spinner covers three sequential phases that can take many seconds:

1. **Model weights loading into GPU** — scales with model size (2–10s)
2. **Whisper encoding audio into mel-spectrogram chunks** — scales with audio length; this is the bulk of the work on long recordings
3. **Whisper decoding the first chunk into text** — the segment callback fires only after this

Everything above is invisible. The bar becomes determinate only when the first segment is decoded, at which point decoding the remaining chunks is fast — so the bar jumps from 0% to near-100% in seconds.

The stage that has progress resolution (`TranscribingAudio` with segment callbacks) is the part that's already going fast. The stages that are slow have no resolution at all.

## What Good Looks Like

- The spinner should end when model loading ends — not persist through audio encoding
- Audio encoding progress should be visible and proportional to recording length
- Decoding progress (segment callbacks) is fine as-is — it's already accurate for what it covers
- If the model was pre-loaded during recording (`spawn_record_start_preload`), the model-load phase should be near-instant and the bar should reflect that

## Notes

- Whisper's encoder phase does not currently emit callbacks — check whether whisper-rs / whisper.cpp exposes encoder progress; if not, elapsed-time estimation based on audio length is an acceptable fallback
- The `set_abort_callback_safe` hook is intentionally disabled on Metal (causes GenericError -6) — any new callback registration must be tested on Metal
- Do not conflate model loading and audio encoding in a single stage label — they are different work with different scaling behaviour
- Pre-load path (`spawn_record_start_preload`) already exists in `scribe.rs`; the progress model should account for the case where the model is already warm
