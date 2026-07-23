---
title: Load Sortformer concurrently with ASR in the Upload (Transcribe) flow
labels: [wayfinder:task]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

## Question

In the Upload flow, `Diarizer::diarize` calls `load_sortformer()` (`services/diarization.rs:262-266`) only after ASR has already produced segments (`services/transcription.rs:157-162, 269-286`), even though loading the Sortformer file (~469MB) has zero dependency on ASR's output — only the later `align_ranges_to_segments` step needs the segments. Can the Sortformer load start as soon as decoded PCM is ready, running concurrently with the ASR pass, instead of waiting behind it?

**Done when:** either (a) Sortformer's model load is kicked off concurrently with (or ahead of) the ASR pass for Upload items, with alignment still correctly waiting on both being ready, or (b) the ticket closes explaining a real constraint that prevents this (e.g. shared resource contention) if one turns up during implementation.

## Comments

- 2026-07-23: Spun off [[29-transcribe-flow-sequential-loading]]. Same shape as the Scribe flow's dual-source-pass ticket ([[34-scribe-parallelize-dual-source-transcription]]) — check whether both tickets touch the same shared inference/diarization resource before implementing either.
