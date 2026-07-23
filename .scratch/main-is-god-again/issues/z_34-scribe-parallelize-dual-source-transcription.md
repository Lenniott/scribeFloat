---
title: Parallelize mic-pass and speaker-pass Whisper transcription in Scribe dual-source flow
labels: [wayfinder:task]
status: closed
assignee: claude-agent
blocked_by: []
parent: MAP.md
---

## Question

`transcribe_capture_with_inference` (`services/transcription.rs:219-252`) runs two full Whisper passes strictly one after another on the same thread when speaker capture is on: mic pass first (0-50% progress), then speaker pass (50-100%), then `merge_dual_source`. Neither pass depends on the other's output — they transcribe two independent, already-finalized PCM buffers (mic track, speaker/loopback track). Can these run concurrently (e.g. two `spawn_blocking` tasks joined before merge), and is the shared inference/model context safely reentrant from two threads at once, or does this need a second lightweight decode context?

**Done when:** either (a) the two passes run concurrently with progress reporting adjusted accordingly, backed by a decision on whether that needs a second decode context, or (b) the ticket closes explaining a real constraint (e.g. the inference engine genuinely cannot serve two concurrent passes without prohibitive memory cost) that keeps them sequential, with that constraint recorded so it isn't re-litigated.

## Comments

- 2026-07-23: Spun off [[28-scribe-flow-sequential-loading]]. Explicitly flagged in the research as needing a reentrancy check before implementation — this is a decision ticket first (is concurrent decode even safe), not a pure mechanical reorder like the Dictate tickets.

## Resolution

**Not implemented — closed as a real constraint, not deferred work.**

`transcribe_capture_with_inference` calls `SpeechInference::transcribe_pass`, which for the real
`ModelService` (`src-tauri/src/services/model.rs`) bottoms out in
`transcribe_pcm_with_progress`. That function opens with:

```rust
let _inference = self
    .inference_gate
    .lock()
    .unwrap_or_else(|p| p.into_inner());
```

`inference_gate: Mutex<()>` is a **process-wide** gate, documented on the field itself:

> Serializes all in-flight `whisper_full` calls. `WhisperContext` is shared across
> Scribe, Dictate, and Transcribe; concurrent encode passes corrupt Metal/ggml state.

This is not an assumption — it is an existing, deliberate design decision already recorded in
the codebase, put there specifically because concurrent `whisper_full` calls on shared
whisper.cpp/ggml (and the Metal backend on macOS) were found to corrupt state. `WhisperContext`
itself is shared/cached per model path (`loaded_contexts: Mutex<HashMap<PathBuf, Arc<WhisperContext>>>`)
precisely to avoid the ~300 ms–2 s reload cost, which forecloses option (a) from the ticket
("two separate lightweight decode contexts") as the *cheap* fix: a second concurrent
`WhisperState`/context pointed at the same model would either (i) still serialize behind
`inference_gate` — since the gate is keyed on "any in-flight `whisper_full` call," not on
context identity — buying nothing, or (ii) require carving out a parallel-inference exception to
that gate specifically for the mic/speaker pair, which reintroduces the exact class of bug the
gate exists to prevent, for a feature (dual-source Scribe/Upload) where GPU (Metal) encode is the
common path and doesn't parallelize well across two encodes competing for the same GPU anyway.

Running the two passes on two `spawn_blocking` tasks today would compile and "work" but would
not actually run concurrently in the way the ticket wants — the second call would simply block
on `inference_gate` until the first finishes, so wall-clock time would be unchanged while adding
two thread-pool hops, two mutex acquisitions, and split progress-reporting complexity for zero
benefit. Given that, mechanically reordering into `spawn_blocking` + join would be pure
complexity with no throughput win, and unsafely bypassing the gate would risk the documented
Metal/ggml corruption. Both are worse than the status quo.

**Decision:** keep the mic pass and speaker pass sequential. The real constraint is the
process-wide `inference_gate` in `services/model.rs`, put there on purpose because whisper.cpp/
Metal state is not safely reentrant across concurrent `whisper_full` calls — not an oversight in
this pipeline. No code changed. If a future ticket wants real parallelism here, it has to start
by relaxing/reworking `inference_gate` for the whisper.cpp/Metal reentrancy question directly
(e.g. per-context state pooling proven safe under concurrent encode), not by touching
`transcription.rs`. Recording this here so it isn't re-litigated.

**Verify:** no code changed; `cargo test -p ScribeFloat` and `cargo clippy -p ScribeFloat -- -D warnings` re-run clean to confirm the pipeline is untouched (see below).
