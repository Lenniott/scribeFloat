# Dictate latency investigation

## macOS merge preparation — 2026-09-20

User requested commits and explicitly deferred Windows. macOS CLI packaging and
bootstrap blockers are fixed; audio-routing helper executable permissions and
packaged filename were also fixed after inspecting the actual app bundle. Rust
formatting gate is clean. Final checks: 373 Rust tests pass (5 ignored), Clippy
and formatting pass. Frontend check and 125 tests passed during review; production
frontend build passes. Release CLI builds and unsigned macOS app bundles; both
packaged helper usage commands run. Native capture smoke remains unverified.

Fetched origin/main is still an ancestor of this branch. Intended changes are
being committed; unrelated main-is-god scratch files and LOG.md remain untouched.
No merge or push requested. See MERGE-REVIEW.md for evidence and remaining scope.

## Implementation state — 2026-09-19

Record integration added in this session: the mic writer tap now feeds both
capture-time ASR and Sortformer. Stop drains capture, finishes workers, and passes
completed mic segments into shared post-capture assembly for final speaker
stamping. Worker failure replays the complete mic WAV. Cancel, WAV-only save, and
shutdown cancel ASR; Stop cancellation also reaches inference and fallback.
System audio is still assembled/transcribed after Stop and uses existing In/Out
channel merging; live ASR currently optimizes the mic only. Upload is unchanged.

Validation: 369 library tests passed (5 hardware tests ignored), binary/doc tests
passed, and Clippy `--lib --examples -- -D warnings` passed. Seven new tests cover
cached mic reuse, absolute speaker alignment, dual-source channel merge, fallback,
empty/plain results, Stop cancellation, and the worker-to-Record assembly path.
No new native microphone/loopback smoke test or Record-specific speed measurement
was performed. Dictate benchmark numbers below must not be described as Record
measurements. Slow Note saving remains separate.

Capture-time ASR is now connected to Dictate using shared
`services/streaming_transcription.rs`. It requires 30 seconds of context and a
confirmed 512 ms pause before splitting; short recordings remain one job. The
complete WAV is preserved for full-pass fallback on worker/VAD/queue failures.
Stop drains the audio writer, joins ASR, uses shared transcript formatting, then
pastes before saving. Live inference has the same int16-quantized PCM as the WAV.
Cancellation shares an abort flag; workers do not emit state or publish results.

The 15-second original fixture now produces exactly the same final text in three
real-time replays as three baseline passes. Final `cargo test -p ScribeFloat`
passes 362 library tests (5 ignored), plus binary/doc targets. Clippy
`--lib --examples -- -D warnings` passes. See `BENCHMARK.md`: 63-second synthetic
replay remaining ASR is 2.135 s versus 4.153 s warm baseline, a 48.6% reduction.
Original short-clip wait is 1.161 s versus 1.123 s, essentially unchanged with
small scheduling overhead. The repeated long fixture has wording differences;
natural-speech accuracy is not established. Native microphone/paste smoke testing
is still for the user; offline ASR and worker lifecycle checks are complete.

The initial aggressive 4-second/256 ms candidate changed a homophone. Bounded
text-prompt carryover, flash attention, and disabling inference VAD were tried
and removed. Do not re-enable those experiments from old `/tmp` binaries/logs.
Temporary `[DEBUG-dictate-timing]` probes have been removed; concise output/save
timings and phrase timing remain. See ADR-0016 and issue 02 for scope and tests.

Historical investigation follows; earlier "not yet implemented" notes describe
the state at those measurements, not the current tree.

User reports slow Dictate and wants to evaluate implementation improvements with
the existing Whisper model. Current scope: timing instrumentation, then the user
runs dev and dictates; inspect that trace before choosing a fix.

Temporary logs in `controllers/dictate.rs` and `services/model.rs` use
`[DEBUG-dictate-timing]`, at info level. A `dictate_timing` span gives each stopped
dictation a run ID. No new log records contain transcript text or audio.

Run from the repo root:

```sh
RUST_LOG=info cargo tauri dev 2>&1 | tee /tmp/scribefloat-dictate-timing.log
```

Measure dispatch/worker wait, WAV finalization/read, transcription preparation,
inference queue, context cache/load, state setup, each inference attempt including
failures, clipboard, history write, and paste/auto-enter. Stop elapsed begins at
the controller's Stop entry, not the physical key event. Output completion is
paste dispatch acknowledgement, not confirmed rendering in the target app.
Worker completion includes the existing 450 ms HUD hold. `ok` on worker completion
only means the worker returned Ok; handled errors/cancellation may also return Ok.

Model warmup/cache and greedy decoding already exist. GPU failures can trigger
CPU retry; no evidence yet that this happens in the user's slow sessions.
Silence-triggered transcription and maximum-length/rolling windows remain
candidates, not accepted implementation decisions. ADR-0003 requires shared
capture infrastructure; ADR-0015 requires indexing after Stop/freeze.

Next: inspect the user's log, distinguish cold/warm runs and retries, then propose
a measured change. Remove temporary instrumentation after the investigation.

## First measured run — 2026-09-17

Log: `/tmp/scribefloat-dictate-timing.log`, run
`89224325-b113-4303-b726-5c8eecd05402`. One completed dictation observed:
21.59 s audio, 14.810 s Stop-to-output acknowledgement, 15.261 s until worker exit.
Dispatch 7 ms; finalize 323 ms; read 73 ms; transcription/formatting 2216 ms
(inference 2048 ms); clipboard/record creation 3 ms; history-write checkpoint
11877 ms; paste/auto-enter 307 ms. Context cached, GPU preference true, one
successful attempt, no inference queue wait or CPU retry in this run.

The history checkpoint includes `history.append` and `note://item-added` emission.
`append` acquires a mutex, ensures history is loaded (including sidecar hydration),
serializes/appends a record and flushes/syncs the file. This trace cannot identify
which substep took the time. Investigate those boundaries next; do not claim
`sync_all` or cache loading is the cause without measurement. Note saving sits
before paste, so background ASR alone cannot remove this measured ~11.9 s delay.
No pipeline behavior changed. Prioritize saving/paste ordering investigation
before a streaming prototype, while preserving durable Note creation.

## Paste ordering fix — 2026-09-17

User authorized addressing the paste bottleneck and explicitly separated the
slow-save investigation. Moved history append/event emission after paste dispatch
and its timing checkpoint. Clipboard-only mode still writes clipboard first;
failed paste still saves the Note; existing history error reporting is retained.
No detached tasks, storage changes, or ASR changes. The existing worker still
waits for saving before dismissing the HUD/returning to Idle, so this change
improves text delivery but does not yet shorten next-dictation availability.

Separate follow-up: [Investigate slow Note saving](issues/01-investigate-slow-note-save.md).
Next measurement: repeat Dictate and verify `output complete` precedes
`history_write`; compare Stop-to-output against the 14810 ms baseline. The
estimated ~3 s delivery is inferred from the earlier run, not yet measured.

Validation: `cargo check --manifest-path src-tauri/Cargo.toml` passed;
`cargo test --manifest-path src-tauri/Cargo.toml --lib controllers::dictate::tests`
passed all 24 existing gesture/state tests. Those tests do not exercise native
paste timing; repeat the instrumented dictation to validate the actual latency.

## Post-change measurement — 2026-09-17

Run `59aa0c5f-5a8b-413c-8a28-de6760c27891`: 314.67 s audio (5m14.67s).
Stop-to-output acknowledgement 16.329 s; history checkpoint ends at 31.323 s;
worker exits at 31.778 s. Finalize 328 ms; audio read 1047 ms;
transcription/formatting 14574 ms (inference 14342 ms); paste/auto-enter 303 ms;
history append/event 15003 ms. Cached model, GPU preference true, no queue wait,
one successful inference attempt, paste_failed=false, no history failure logged.

Validated: output now precedes saving, keeping roughly 15 s of saving off the
paste path in this run. Different audio lengths prevent a like-for-like speedup
claim against the earlier 21.59 s recording. Remaining delivery delay is dominated
by post-Stop ASR. Background phrase transcription remains a candidate; aggregate
inference processed ~21.9 recorded audio seconds per wall second, but this does
not establish performance/accuracy for short chunks or the proportion of silence.

An intervening 3.145 s run ended after transcription in 1.148 s without output or
history checkpoints. The current log does not distinguish empty filtered segments
from cancellation, so it is not evidence of successful paste or save.
