---
title: Delete Dictate temp WAV right after PCM read instead of after paste
labels: [wayfinder:task]
status: closed
assignee: claude-agent (worktree agent-ab5cdd260cc85ba7f)
blocked_by: []
parent: MAP.md
---

## Question

In `do_transcription` (`dictate.rs:802`), the temp WAV file is deleted only after paste completes, but nothing downstream of the PCM read (`read_wav_mono_f32`) re-reads that file. Deleting it as soon as the PCM buffer is safely in memory removes a needless dependency on the paste step completing first.

**Done when:** temp WAV deletion happens immediately after `pcm_16k` is read successfully, not gated on paste; error handling for delete failure is unchanged (still non-fatal, still logged). `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass; approach recorded in Resolution.

## Comments

- 2026-07-23: Spun off [[27-dictate-flow-sequential-loading]]. Lowest-risk of the four Dictate tickets — pure reordering, no new concurrency.

## Resolution

**Deviated from the literal "immediately after `pcm_16k` is read" wording** — flagging this explicitly rather than guessing silently. Reading `pcm_16k` successfully is not actually the last read of `wav_path`: two failure branches between the PCM read and the success path — model-unavailable (`self.model.model_available(&model_path)` failing) and the transcription call itself failing — both call `self.salvage_dictate_wav(&wav_path)`, which copies the temp WAV to a failures directory for user recovery (`OutputService::salvage_dictate_wav`, see `context/README.md` "hard ownership rules" — `OutputService` owns dictate failure salvage). Deleting the file right after the PCM read would silently break salvage on those two paths, since `remove_file` would already have removed the source the salvage copy needs.

Instead, moved the delete to immediately after the last point that can still need `wav_path` for salvage: right after `segments.is_empty()` returns false (i.e., transcription has fully succeeded and no further code path reads or salvages the file), and well before clipboard write, history append, or paste. This still removes the WAV file's lifetime from gating on paste completion — the stated goal — while preserving salvage-on-failure. The early-return branches (`MIN_PCM_SAMPLES_16K` too-short check, abort-flag, empty-segments) already deleted the WAV at their own point and are unchanged.

Also removed the two now-redundant `delete_dictate_wav` calls further down (clipboard-write-failure branch and the final call before the Done-state block, per ticket 32's resolution) since the file is already gone by the time either would have run. Delete-failure handling is unchanged: `delete_dictate_wav` still does a fire-and-forget `let _ = std::fs::remove_file(path)`, non-fatal, no new logging added or removed.

**Verify:** `cargo test -p ScribeFloat` → all `controllers::dictate::tests` and `services::output::tests::salvage_dictate_wav_moves_to_failures_dir` green (salvage path untouched by this change). `cargo clippy -p ScribeFloat -- -D warnings` clean.
