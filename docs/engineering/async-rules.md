# Async Rules and State Machines

> Load this when touching controller threading, Whisper inference paths, or audio callbacks.

---

## Async rules

- Long CPU-bound work (Whisper, WAV merge) runs in `tokio::task::spawn_blocking`.
- Audio stream callbacks run on cpal's thread — never await or block inside them.
- The macOS paste path (`run_on_main_sync`) must not be called from a Tauri async command handler — it will deadlock. Use the `finish_session_async` pattern from `dictate_stop` command as the reference.
- Tauri `#[tauri::command]` functions that call blocking code (e.g. `stop_and_take` which drains an audio channel) **must be `async`** and wrapped in `tokio::task::spawn_blocking`. Sync commands run on the main thread — blocking there hangs the entire UI event loop.

---

## State machines

Each controller exposes a state via a `Mutex<Inner>`. Methods lock, check the current state, act, and release. **Never hold a lock across a blocking call** (Whisper, file I/O). If you need to do blocking work, extract the data under lock, drop the lock, then do the work.

```
ScribeController:     IDLE → RECORDING → TRANSCRIBING → DONE | NO_MODEL | ERROR
DictateController:    IDLE → RECORDING → TRANSCRIBING → PASTING → DONE | ERROR
TranscribeController: IDLE → TRANSCRIBING → DONE | ERROR
```

State lives entirely inside `Inner`. Controllers never expose `Inner` directly.
