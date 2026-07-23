---
title: Sequential-loading habits in the Scribe (Record) flow
labels: [wayfinder:research]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

Same question as [[27-dictate-flow-sequential-loading]], for the Scribe/Record flow instead: `controllers/scribe.rs` end to end — session start → recording → stop → transcription/diarization pipeline → history write → UI update. Where is work done sequentially that could be parallel, prefetched, or backgrounded? Diarization (Sortformer) adds a step Dictate doesn't have — check whether diarization and transcription could overlap, or whether one strictly waits on the other for a real reason.

**Done when:** trace report (file:line references) written to `research/scribe-flow-sequential-loading.md` and linked here, same shape as ticket 27.

## Resolution

Traced — see [research/scribe-flow-sequential-loading.md](../research/scribe-flow-sequential-loading.md). Good news first: live diarization and Whisper preload during capture are **already** backgrounded correctly (diarization starts concurrently with mic capture; preload fires during recording) — this flow does not repeat the startup-style habit at that layer. It does recur in exactly one place: `transcribe_capture_with_inference` (`services/transcription.rs:219-252`) runs two independent Whisper passes (mic track, speaker track) strictly one after another on one thread, though neither depends on the other's output.

Spun off:
- [Parallelize dual-source Whisper passes](./issues/34-scribe-parallelize-dual-source-transcription.md)

No other follow-ons — `write_outputs`' chain and the mic-finalize→diarization-finish ordering were confirmed as genuine dependencies, not false-sequential habits.

## Comments

- 2026-07-23: Resolved by background Explore agent. Agent's sandbox couldn't write files directly (read-only exploration tooling) — findings were returned in the completion report and the orchestrating session wrote the research file and this resolution from that report, unmodified in substance.
