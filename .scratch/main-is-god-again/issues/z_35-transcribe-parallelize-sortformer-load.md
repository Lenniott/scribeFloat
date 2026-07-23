---
title: Load Sortformer concurrently with ASR in the Upload (Transcribe) flow
labels: [wayfinder:task]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

In the Upload flow, `Diarizer::diarize` calls `load_sortformer()` (`services/diarization.rs:262-266`) only after ASR has already produced segments (`services/transcription.rs:157-162, 269-286`), even though loading the Sortformer file (~469MB) has zero dependency on ASR's output — only the later `align_ranges_to_segments` step needs the segments. Can the Sortformer load start as soon as decoded PCM is ready, running concurrently with the ASR pass, instead of waiting behind it?

**Done when:** either (a) Sortformer's model load is kicked off concurrently with (or ahead of) the ASR pass for Upload items, with alignment still correctly waiting on both being ready, or (b) the ticket closes explaining a real constraint that prevents this (e.g. shared resource contention) if one turns up during implementation.

## Resolution

Took option (a) — unlike ticket 34, this one doesn't share `ModelService.inference_gate`: Sortformer runs through `parakeet_rs::sortformer::Sortformer`, loaded fresh per call (`DiarizationService::load_sortformer`, `services/diarization.rs:205`) with no shared mutex or state with Whisper's inference path. That made real concurrency safe to implement rather than just a decision to decline.

In `services/transcription.rs::run_post_capture_transcription_with_inference`, when an on-demand diarization pass is coming (`will_diarize_after_asr`), the function now uses `std::thread::scope` to spawn `diarizer.diarize(mic_pcm)` on a scoped thread while the ASR pass (`transcribe_capture_with_inference`) runs on the calling thread — `Diarizer: Sync` and `mic_pcm_16k: &[f32]` are both safely shareable across the scope, so no `'static`/ownership gymnastics were needed. The scope joins the diarization handle right after ASR returns, producing `diarize_result: Option<Result<Vec<DiarizationRange>>>`.

`build_speaker_result` no longer calls `diarizer.diarize()` itself for the on-demand case — it now takes the precomputed `Option<Result<Vec<DiarizationRange>>>` and either uses the ranges or logs-and-degrades on error, exactly matching prior behavior. `LiveRanges` (Record) and no-evidence paths are untouched, since only the `DiarizeOnDemand` (Upload) branch ever dispatched a concurrent pass.

**Caveat flagged, not resolved:** whether Whisper (Metal) and ONNX Runtime (Sortformer, likely CoreML/Metal-backed on macOS) genuinely run concurrently on the GPU without contention, or just interleave with no net time saved, isn't something I can verify without profiling on real hardware with real models — this environment has no working model files to run an actual dual-pass Upload item through. The change is safe (no shared mutable state, existing tests for `upload_diarizes_on_demand_and_aligns` and diarization-failure-degrades pass unchanged) but the *performance* claim is unverified. Worth a real-world timing check before calling this "done" in the product sense.

**Verify:** `cargo build` clean. `cargo test -p ScribeFloat` → 350 passed, 0 failed, including `upload_diarizes_on_demand_and_aligns` and `upload_diarizer_failure_degrades_to_plain_transcript` (both exercise this exact path, unchanged pass/fail behavior). `cargo clippy -p ScribeFloat -- -D warnings` clean.

## Comments

- 2026-07-23: Spun off [[29-transcribe-flow-sequential-loading]]. Same shape as the Scribe flow's dual-source-pass ticket ([[34-scribe-parallelize-dual-source-transcription]]) — check whether both tickets touch the same shared inference/diarization resource before implementing either.
- 2026-07-23: Implemented directly against `release/0.3` (not via the worktree agent, which never got this far before hitting the stale-base blocker).
