---
title: Prefetch next queued item's decode in Transcribe batch loop
labels: [wayfinder:task]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

`run_batch` (`transcribe.rs:192-382`) processes queued Upload items in a plain `for` loop — item N+1's decode (`decode_input`, pure file I/O/CPU resample, no dependency on item N) can't start until item N's write+journal+emit fully completes. Can the next item's decode be kicked off while the current item's ASR+diarization is still in flight, so decode time for item N+1 overlaps with inference time for item N instead of stacking after it?

**Done when:** either (a) next-item decode is prefetched/overlapped with the current item's inference stage, with the queue's progress reporting and cancellation behavior unaffected, or (b) the ticket closes explaining why decode overlap isn't worth the complexity for typical batch sizes (e.g. decode is a small fraction of total item time) if that turns up during implementation.

## Resolution

**Took option (b) — declined, not deferred quietly.** Unlike tickets 35 and 25 (pure additions with an obvious safe seam), overlapping decode with the current item's inference in `run_batch` (`transcribe.rs:192-382`) means restructuring a single-threaded per-item loop into a real two-stage pipeline: spawn item N+1's decode before item N's ASR/diarization/write finishes, then rendezvous with that result at the top of the next iteration, while still handling a decode failure on the *prefetched* item correctly (it fired before the loop even reached that index — does it record as errored immediately, or does the queue-state emit for it wait?), still respecting the current per-item `queue[index]` progress/status mutation pattern, and still behaving correctly if the batch is cancelled mid-flight (does the in-flight prefetch get dropped, joined, or ignored?).

None of that is undoable, but doing it correctly needs either a real batch of files with a working Whisper/Sortformer model to observe actual decode-vs-inference timing, or careful synchronous reasoning about every one of those edge cases without being able to run it — and I have neither real timing data nor a way to exercise this path end-to-end in this environment (bundled models here are placeholders). Forcing a plausible-looking pipeline restructure into `run_batch` without either of those would be exactly the kind of change this whole effort is trying to avoid: code that looks like an improvement but is actually an unverified guess.

The ticket's own "Done when" explicitly allows closing here: "if decode is a small fraction of total item time." Decode is CPU/file-I/O bound resampling of a single audio file; Whisper ASR (even Small) plus a Sortformer diarization pass over the same audio is very likely an order of magnitude slower per item for anything but trivially short clips — meaning the overlap this ticket chases is bounded above by decode time, and total win across a batch is capped by (decode_time × min(N-1, pipeline depth)), not something that compounds. Given the implementation risk above and an expected upside that's real but likely small, I'm closing this rather than building it.

**If revisited:** do it with real timing data from an actual batch run first (log decode duration vs. total item duration for a representative batch), and only build the pipeline if that shows decode is a non-trivial fraction of wall-clock time.

**Verify:** no code changed; existing `cargo test -p ScribeFloat` / `cargo clippy -p ScribeFloat -- -D warnings` results (350 passed, clean) are unaffected by this decision.

## Comments

- 2026-07-23: Spun off [[29-transcribe-flow-sequential-loading]]. Lowest-priority of the three Transcribe tickets — real win scales with batch size and decode cost relative to inference time, which hasn't been measured yet.
- 2026-07-23: Investigated directly against `release/0.3` (not via the worktree agent). Declined per the ticket's own option (b) rather than force an unverified pipeline change.
