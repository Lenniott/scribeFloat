---
title: Sequential-loading habits in the Dictate flow
labels: [wayfinder:research]
status: closed
assignee: claude
blocked_by: []
parent: MAP.md
---

## Question

This whole investigation started because the human said "the dictate process was slow" — but we've only traced app *startup* so far, not the live Dictate round trip. Where in the hotkey-press → capture → transcribe → paste/insert path does the code do things sequentially (one await after another) that could be parallel, prefetched, or backgrounded — excluding the raw model inference time itself (that's fixed cost, out of scope per the human's steer)?

Trace `controllers/dictate.rs` end to end: key listener → `start_recording`/capture setup → stop → transcription pipeline → output (paste/insert). Note any related Known issue already on this map: [Onboarding Dictate practice pays cold Whisper load](../KNOWN-ISSUES.md) (Whisper preload only starts when recording starts — first practice capture waits ~15s). Is that pattern (waiting to kick off work until the last possible moment, instead of during idle time) repeated elsewhere in the same flow?

**Done when:** a trace report (file:line references) of every sequential step in the Dictate round trip, flagging which are genuinely dependent (must happen in order) vs. independent (could run concurrently or be started earlier), written to `research/dictate-flow-sequential-loading.md` and linked here.

## Resolution

Traced — see [research/dictate-flow-sequential-loading.md](../research/dictate-flow-sequential-loading.md). The whole round trip lives in `dictate.rs` (no frontend involvement). Confirmed the known "wait until the last possible moment" pattern recurs here too: Whisper preload only starts after `Recording` is set inside `start()`, not at key-down or HUD-open. Also found three other sequencing issues unrelated to model load time: blind `sleep(50ms)` polling waits instead of real completion signals on main-thread hops; history-append and clipboard-write run sequentially with no dependency between them; temp-WAV deletion waits until after paste despite no dependency on it.

Spun off:
- [Start Dictate Whisper preload earlier](./issues/30-dictate-preload-earlier.md)
- [Replace blind sleeps with real completion signals](./issues/31-dictate-replace-blind-sleeps.md)
- [Reorder clipboard write vs history append](./issues/32-dictate-reorder-clipboard-history.md)
- [Delete temp WAV right after PCM read](./issues/33-dictate-delete-temp-wav-early.md)

## Comments

- 2026-07-23: Ticketed as part of the broader "root out the sequential-loading habit" effort. Resolved by background Explore agent; findings verified against current line numbers by the agent, spot-checked by the orchestrating session.
