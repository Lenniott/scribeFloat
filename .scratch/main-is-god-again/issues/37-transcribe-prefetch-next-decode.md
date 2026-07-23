---
title: Prefetch next queued item's decode in Transcribe batch loop
labels: [wayfinder:task]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

## Question

`run_batch` (`transcribe.rs:192-382`) processes queued Upload items in a plain `for` loop — item N+1's decode (`decode_input`, pure file I/O/CPU resample, no dependency on item N) can't start until item N's write+journal+emit fully completes. Can the next item's decode be kicked off while the current item's ASR+diarization is still in flight, so decode time for item N+1 overlaps with inference time for item N instead of stacking after it?

**Done when:** either (a) next-item decode is prefetched/overlapped with the current item's inference stage, with the queue's progress reporting and cancellation behavior unaffected, or (b) the ticket closes explaining why decode overlap isn't worth the complexity for typical batch sizes (e.g. decode is a small fraction of total item time) if that turns up during implementation.

## Comments

- 2026-07-23: Spun off [[29-transcribe-flow-sequential-loading]]. Lowest-priority of the three Transcribe tickets — real win scales with batch size and decode cost relative to inference time, which hasn't been measured yet.
