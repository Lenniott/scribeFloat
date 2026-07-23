---
title: Reorder or parallelize Dictate clipboard-write vs history-append
labels: [wayfinder:task]
status: closed
assignee: claude-agent (worktree agent-ab5cdd260cc85ba7f)
blocked_by: []
parent: MAP.md
---

## Question

In `do_transcription` (`dictate.rs:802`), history append (`dictate.rs:931`) and clipboard write (`dictate.rs:940`) both depend only on the final transcribed `text`, not on each other, but run sequentially with history first — and clipboard write is what actually gates the paste the user is waiting on. Should these run concurrently, or should clipboard write (and paste) simply move ahead of history append since paste is user-visible and history append is not?

**Done when:** either (a) clipboard write/paste happens before or concurrently with history append rather than strictly after, with history append's own error handling unaffected, or (b) the ticket closes explaining why the current order is actually required (e.g. history append must record the id before paste for some downstream reason) if research turns that up during implementation.

## Comments

- 2026-07-23: Spun off [[27-dictate-flow-sequential-loading]]. Likely the single highest user-perceptible win of the four Dictate tickets, since it sits directly on the "how long until my text appears" path.

## Resolution

Took option (a) reordering rather than concurrency: moved the clipboard `write_text` call ahead of `write_dictate_history_entry` in `do_transcription`. No downstream code reads `history_write_failed` before the clipboard write (it's only surfaced in the final `Done` state event, alongside `paste_failed`), so reordering doesn't change either call's error handling — clipboard failure still short-circuits into `set_error_state` + early return, history failure is still logged and folded into `history_write_failed` for the Done event.

Chose plain reordering over running the two concurrently: `do_transcription` already runs inside a single `spawn_blocking` closure (not `async`), both calls are cheap synchronous I/O (an in-memory clipboard write and a small JSON-lines append), and spinning up a thread/task pair to shave a few sub-millisecond disk writes off the "text appears" path isn't worth the added complexity or a new failure mode (e.g. a history-write panic in a spawned task needing its own catch). Clipboard write is what actually unblocks the user-visible paste, so putting it first delivers the same practical latency win as concurrency would, with a one-line diff.

Removed the two calls to `self.delete_dictate_wav(&wav_path)` that used to run on a clipboard-write failure and after paste completed — both are now redundant since ticket 33 (below) deletes the temp WAV immediately after a successful transcription, before either clipboard write or history append run.

**Verify:** `cargo test -p ScribeFloat` → all `controllers::dictate::tests` and `services::output::tests::*dictate_history*` green. `cargo clippy -p ScribeFloat -- -D warnings` clean.
