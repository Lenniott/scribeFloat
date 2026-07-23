---
title: Sequential-loading habits in the Transcribe (Upload) flow
labels: [wayfinder:research]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

Same question as [[27-dictate-flow-sequential-loading]] and [[28-scribe-flow-sequential-loading]], for the Transcribe/Upload flow: `controllers/transcribe.rs` end to end — file picked → import/decode → transcription/diarization pipeline → history write → UI update. Where is work sequential that could be parallel, prefetched, or backgrounded?

**Done when:** trace report (file:line references) written to `research/transcribe-flow-sequential-loading.md` and linked here, same shape as tickets 27/28.

## Resolution

Traced — see [research/transcribe-flow-sequential-loading.md](../research/transcribe-flow-sequential-loading.md). The batch queue is a single `for` loop where item N+1's decode can't start until item N's write+journal+emit fully finishes, despite decode being pure file I/O independent of the previous item. Found the startup habit's VAD-hash issue recurring here too, but worse: `transcribe_pcm_with_progress` re-hashes the Whisper model with a synchronous SHA-256 on *every single batch item's critical path*, not just once per launch. Also found Sortformer diarization's model load (`load_sortformer`) sits strictly after ASR completes, even though the file read has zero dependency on ASR's output — only the final alignment step needs ASR's segments.

Spun off:
- [Load Sortformer concurrently with ASR in Upload flow](./issues/z_35-transcribe-parallelize-sortformer-load.md)
- [Stop re-hashing Whisper model per batch item](./issues/z_36-transcribe-cache-whisper-hash.md)
- [Prefetch next queued item's decode](./issues/z_37-transcribe-prefetch-next-decode.md)

## Comments

- 2026-07-23: Resolved by background Explore agent; file written directly by the agent (unlike ticket 28's agent, this one had write access).
