---
title: Replace blind sleep(50ms) main-thread-hop waits in Dictate with real completion signals
labels: [wayfinder:task]
status: closed
assignee: claude-agent (worktree agent-ab5cdd260cc85ba7f)
blocked_by: []
parent: MAP.md
---

## Question

`spawn_dictate_window_and_start_inner_async` (`dictate.rs:503-511`), `spawn_dictate_hold_while_held` (`dictate.rs:459`), and `spawn_dictate_hold_immediate_stop` (`dictate.rs:492`) all use a hardcoded `sleep(50ms)` to wait for a `run_on_main_thread` result instead of awaiting the callback's completion directly. This is a blind poll, not a real signal — it either wastes 50ms when the main-thread hop finishes faster, or (worse) can read a stale/incomplete result if the hop takes longer than 50ms on a loaded system.

**Done when:**
1. All three call sites await the actual result of the main-thread hop (e.g. via a oneshot channel/callback already available from `run_on_main_thread`, or an equivalent completion signal) instead of a fixed sleep.
2. No change in observed behavior for the fast case; the slow-system case no longer risks reading a stale result.
3. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass; approach recorded in Resolution.

## Comments

- 2026-07-23: Spun off [[27-dictate-flow-sequential-loading]].

## Resolution

`run_on_main_thread` itself has no completion signal (it schedules `f` and returns immediately, or returns `Err` if scheduling fails outright). Added a `tokio::sync::oneshot::channel` inside the shared helper `capture_paste_target_then_open_overlay`, which is now `async fn ... -> Result<(), String>`: the main-thread closure sends its result down `tx` as its last action, and the async fn awaits `rx` instead of returning an `Arc<Mutex<Result<...>>>` for callers to poll after a fixed sleep. If `run_on_main_thread` fails to schedule at all, the error is returned immediately without waiting on the channel; if the sender is dropped without sending (closure panicked), `rx.await` surfaces that as an `Err` rather than hanging.

All three call sites now `.await` the real result directly:
- `spawn_dictate_window_and_start_inner_async` (the shared inner routine used by both `spawn_dictate_window_and_start` / Toggle, and `spawn_dictate_hold_immediate_stop`, which delegates to it) — dropped the `sleep(50ms)` + `Mutex` poll, awaits the helper directly.
- `spawn_dictate_hold_while_held` — same swap; the two `hold_start_cancel` checks (before and after inspecting the open result) are unchanged in position, they just now bracket a real await instead of a blind sleep.

**Verify:** `cargo test -p ScribeFloat` → all `controllers::dictate::tests` green (state-machine tests are synchronous and untouched by this change). `cargo clippy -p ScribeFloat -- -D warnings` clean.
