# scribefloat — Working Guide

> How to build, debug, and extend this codebase.
> For what the app does and how it is designed, read `context/README.md` first.

---

## Read before touching anything

```
context/README.md          ← Start here. Reading order and behaviour quick reference.
context/architecture.md    ← Layer rules, call chain, C4 diagrams.
context/action-flows.md    ← Step-by-step flows for each workflow.
context/components.md      ← UI component catalogue.
docs/README.md             ← Docs index (History UI guide, backlog).
docs/history-ui-review.md  ← Required before History list/detail UI changes.
docs/backlog.md            ← Deferred follow-ups.
```

---

## Design system & UX playbook

Before writing any frontend code, query the design skill:

```
context/design-skill/SKILL.md   ← Start here. Commands and query patterns.
context/design-skill/query.py   ← CLI query tool (run from any directory).
```

Quick start:

```bash
python3 context/design-skill/query.py ds toc              # design token / component index
python3 context/design-skill/query.py ux toc              # UX playbook chapter index
python3 context/design-skill/query.py ds get components.button   # button spec
python3 context/design-skill/query.py ds get tokens.colors.dark  # full dark palette
python3 context/design-skill/query.py search "waveform"   # search both files
```

Always pull the relevant token/component spec before writing Tailwind classes. The design system owns colour, spacing, radius, and typography — do not guess or hardcode values.

**UI enforcement skill** (typography and future aspects): `.cursor/skills/ui-enforcement/SKILL.md` — load the relevant reference chapter, use `sf-*` role classes from `src/app.css`, run `npm run check:ds`. Typography migration inventory: `docs/typography-audit.md`.

---

## Build and test

```bash
cargo tauri dev                          # Start dev build (full app)
cargo test -p scribefloat                   # Run unit tests
cargo clippy -- -D warnings              # Must pass before committing
cargo check                              # Fast compile check (no link step)
```

If `cargo tauri dev` fails with a missing asset error, check that frontend HTML files under `src/ui/panels/` exist.

---

## Layer rules (enforced — do not break)

```
panel (HTML/JS)
  → command (IPC translation only — no logic)
    → controller (owns state machine, orchestrates services)
      → service (stateless or singleton, instantiated once in lib.rs)
        → platform (OS-specific only, behind #[cfg(target_os)])
```

**Commands** (`src-tauri/src/commands/`) — translate between JS types and Rust types and call one controller method. Nothing else. No business logic here.

**Controllers** (`src-tauri/src/controllers/`) — own the state machine (`Arc<Mutex<Inner>>`). Orchestrate calls to services. Do not open audio streams, write files, or check permissions directly.

**Services** (`src-tauri/src/services/`) — singletons created in `lib.rs::run()` and passed down. Never instantiated inside a controller.

**Platform** (`src-tauri/src/platform/`) — the only place `#[cfg(target_os = "...")]` is allowed. Everything above is platform-agnostic.

**Hard ownership rules:**
- `HistoryService` owns the structured record store (`{save_folder}/history.jsonl`): append, compact, delete (tombstone). Every transcript-bearing flow (Scribe, Dictate, Transcribe) always appends a record here on completion.
- `OutputService` owns markdown rendering (pure free functions: `render_transcript_markdown`, `render_transcript_body`, `count_words`, `render_from_record`) and durable file I/O: `.md` writes, session manifests, post-transcription cleanup, dictate failure salvage, legacy reads, and delete primitives. It no longer owns the structured record store. Markdown output is **opt-in** via `save_transcripts_as_markdown` (default off) and gates Scribe and Transcribe only — Dictate never writes `.md`.
- `AudioService` opens audio streams **and** streams capture to checkpointed temp/session WAV files (16 kHz writer thread). This keeps RAM flat during long recordings; do not accumulate PCM in controllers.
- `PermissionsService` is the only code that checks OS permissions.

---

## Scribe UI — manual recording start (do not regress)

The Scribe webview is **prewarmed at startup** (`prewarm_scribe_window` in `src-tauri/src/lib.rs`). Opening Scribe (tray, hotkey, or menu) must **not** call `scribe_start` — the user starts capture with **Start Recording** in `scribe.svelte`.

**Rules for agents:**

- **Never** auto-invoke `scribe_start` when the Scribe window is shown, focused, or prewarmed.
- **Never** reintroduce `autoStart` / `autoStartRecording` / `scribe://open-requested` arming for recording.
- **Record again** (processing screen) returns to the idle Scribe UI; the user taps **Start Recording** again (error-state **Try again** in `scribe.svelte` may call `startRecording` directly).

---

## History UI (do not regress)

Read **`docs/history-ui-review.md`** before changing History screens or components.

**Rules for agents:**

- **List vs detail** are separate full-height modes — no `SplitPane`. Detail opens from the list row title or **View** (`Eye`); list and filter tabs stay hidden until **Close**.
- **Delete** only on `HistoryListCard` for store records; confirm modal on `history.svelte` — cards emit events, never call `history_delete` directly.
- **Detail footer** uses `PanelFooter` (flex `shrink-0` below scroll). Do not add `FixedFooterBar` to History detail.
- **List card**: title is a `<button>`; action icons are siblings with `stopPropagation` — no nested buttons.
- **Legacy ids** (`md::`, `dictate::`): read-only in UI (no delete/export).
- **Scribe history metadata**: `speaker_capture` = `scribe_capture_speaker` config at write time; `dual_source` = speaker PCM was merged for transcription — do not set both from the same boolean.

---

## How to add a new IPC command

1. Add a `#[tauri::command]` fn to the relevant file in `commands/`.
2. Register it in the `tauri::generate_handler![]` macro in `lib.rs`.
3. If the command accepts user-supplied strings (paths, hotkeys, names), validate them in the command fn before passing to the controller. Reject early with a descriptive `Err(String)`.
4. Do not add logic to the command fn — call one controller method and return its result.

---

## How to add a new feature

1. Check `context/action-flows.md` — if the behaviour is not described there, confirm scope before building.
2. Decide which layer it belongs to (controller, service, or platform adapter).
3. If it requires OS-specific behaviour, define a trait in `platform/mod.rs` and implement it per platform. The controller calls the trait, never the concrete type.
4. If it writes durable transcript **files** (`.md`, WAV cleanup, manifests), route through `OutputService`.
5. If it appends or updates the structured session **record** (`history.jsonl`), route through `HistoryService` (controllers orchestrate; never append JSONL from the frontend).
6. If it needs config, add a field to `Config` in `types.rs` with a `#[serde(default)]` so existing config files keep loading.

---

## How to investigate a bug

| Symptom | Start here |
|---------|-----------|
| Audio not capturing or wrong device | `services/audio.rs` → `MicSession` |
| Transcription wrong or failing | `services/model.rs` → `transcribe_pcm_with_progress` |
| Dual-source merge / mic bleed issue | `services/model.rs` → `merge_dual_source` |
| Speaker channel hallucinating ("Thank you." etc.) | `controllers/scribe.rs` → `pcm_rms` (silence gate) and `filter_hallucination_phrases` |
| Speaker capture not toggling or output device not restoring | `controllers/scribe.rs` → `toggle_speaker_capture`; check `restore_output_device` |
| Loopback device not found | `platform/mod.rs` → `loopback_device_and_config`; check BlackHole install or `preferred_speaker_device` config |
| Transcript paragraphs not grouping correctly | `services/output.rs` → `write_transcript`; check `MERGE_GAP_MS` and `speaker_source_prefix` |
| File not saved or wrong path | `services/output.rs` |
| History record missing or wrong after a session | `services/history.rs` → `append` / `compact` |
| History list wrong, merge/dedupe incorrect, or delete fails | `controllers/history.rs` |
| History detail layout, delete placement, or prev/next wrong | `docs/history-ui-review.md` → `history.svelte`, `HistoryDetailPane`, `HistoryListCard` |
| History chips wrong (dual source vs speaker capture) | `types.rs` `HistoryRecord::from_scribe` + `controllers/scribe.rs` write path |
| UI shows stale state | `commands/` fn for that panel → check emitted events |
| Hotkey not triggering | `lib.rs::run()` → `global_shortcut.on_shortcut` |
| Config not persisting | `services/config.rs` → `update()` and `save()` |
| macOS paste failing in Dictate | `platform/paste_impl.rs` |
| Permission check wrong | `platform/permissions_impl.rs` |

When a bug is in a tight loop (audio callback, transcription progress): add a `// BUG:` comment describing the issue, do not patch blindly. Audio callback code in particular has timing constraints — understand the thread model before changing it.

---

## Debugging Whisper transcription

Whisper runs inside `tokio::task::spawn_blocking`. If you add logging or timing to the transcription path, use `eprintln!` or `std::time::Instant` — `tracing` spans do not propagate into blocking threads without extra setup.

The `on_tick` callback is called per Whisper segment. If progress appears stuck, the model is still running — Whisper does not yield between segments on a chunk.

---

## State machines

Each controller exposes a state via a `Mutex<Inner>`. The states are:

- **Scribe**: IDLE → RECORDING → TRANSCRIBING → DONE / NO_MODEL
- **Dictate**: IDLE → RECORDING → TRANSCRIBING → PASTING → IDLE
- **Transcribe**: IDLE → TRANSCRIBING → DONE / ERROR

State lives entirely inside `Inner`. Methods on the controller lock, check the current state, act, and release. Never hold a lock across a blocking call (Whisper, file I/O). If you need to do blocking work, extract the data under lock, drop the lock, then do the work.

---

## Async rules

- Long CPU-bound work (Whisper, WAV merge) runs in `tokio::task::spawn_blocking`.
- Audio stream callbacks run on cpal's thread — never await or block inside them.
- The macOS paste path (`run_on_main_sync`) must not be called from a Tauri async command handler — it will deadlock. Use `finish_session_async` pattern from `dictate_stop` command as the reference.
- Tauri `#[tauri::command]` functions that call blocking code (e.g. `stop_and_take` which drains an audio channel) **must be `async`** and wrapped in `tokio::task::spawn_blocking`. Sync commands run on the main thread — blocking there hangs the entire UI event loop.

---

## macOS main-thread / dispatch-queue rules

These rules exist because macOS 13+ strictly enforces which APIs may be called from which thread. Violations produce `dispatch_assert_queue_fail` (SIGTRAP / EXC_BREAKPOINT) and kill the process silently — no panic, no error message.

**APIs that require the main dispatch queue:**
- `enigo` keyboard simulation (`CGEventCreateKeyboardEvent`) — used in `platform/paste_impl.rs`
- Any AppKit window operation: `show()`, `hide()`, `set_focus()`, `set_position()`
- `TSMGetInputSourceProperty` — called internally by `rdev` on macOS for every key event

**Rules:**
1. Never call `paste_text()` or `send_enter()` from a tokio thread or `spawn_blocking`. Always dispatch via `app.run_on_main_thread(|| { ... })`. If you need the result back, bridge with a `std::sync::mpsc::channel`.
2. Never call `window.show()`, `window.hide()`, or `window.set_focus()` from any thread other than the main thread. Use `app.run_on_main_thread` for all window visibility operations outside of Tauri command handlers (which already run on the main thread).
3. **Never use `rdev::listen` on macOS.** rdev calls `TSMGetInputSourceProperty` for every key event on its listener thread, which asserts on macOS 13+. Use the `CGEventTap` implementation in `platform/key_listener.rs` instead — it reads raw keycodes with no string conversion.
4. **`set_focus()` on a window while the app is in `.accessory` activation policy kills the process.** An app enters accessory mode via `setActivationPolicy(.accessory)` or `set_dock_visibility(false)`. Never call `set_focus()` on a window when the app may be in accessory mode. The dictate HUD intentionally never calls `set_focus()` for this reason.
5. **`LSUIElement = true` in `Info.plist` is necessary but not sufficient:** Tao defaults to Regular activation at launch, so the Dock stays visible until `set_dock_visibility(false)` runs. **`setup` ends with `sync_activation_policy`** after prewarming windows so a tray-only start hides the Dock. Do not remove that call. Still avoid arbitrary `set_has_visible_windows(false)` elsewhere without respecting visibility logic — especially anywhere an `always_on_top` HUD might interact oddly with focus (`DictateController` deliberately skips `sync_activation_policy` on HUD lifecycle).

**Paste focus timing:**
When simulating a paste (Cmd+V) after dictation, the dictate HUD window must be hidden *before* the keypress is simulated. If the HUD is still visible, it holds focus and Cmd+V fires into the HUD rather than the previously active app. Hide first, sleep ~150ms for the OS to restore focus, then paste. See `DictateController::paste_on_main_thread`.

---

## Audio drain / MicSession rules

`MicSession::stop_and_take()` blocks until the audio sender is dropped. On macOS, cpal tears down its CoreAudio stream **asynchronously** — the callback closure (which holds the `Sender`) may not be dropped immediately after `drop(_stream)`. For short recordings (< ~2 s), this causes an indefinite hang.

**Rule:** Always use `recv_timeout` in the drain loop, not `recv`. The current implementation uses a 200ms timeout — if no chunk arrives within 200ms after the stream is dropped, the drain is considered complete. Never change this back to blocking `recv()`.

---

## Config changes

- All config fields must have `#[serde(default)]` or `#[serde(default = "fn")]` so old config files deserialise cleanly after a schema change.
- If a new field changes existing behaviour, document the default clearly in the field's doc comment.
- `ConfigService::update()` saves atomically (temp file + rename). Always use it — never write `config.json` directly.

---

## Platform code rules

- `#[cfg(target_os = "macos")]` and `#[cfg(target_os = "windows")]` belong only inside `src-tauri/src/platform/`.
- If you add macOS-specific code, add a Windows stub (even if it just returns `Ok(())` or `false`) so the project compiles on both platforms.
- macOS FFI (`objc2`, `dispatch2`) uses `unsafe`. Before adding unsafe blocks: check for a safe abstraction in those crates first, and add a comment explaining why unsafe is necessary.

---

## Things to check before committing

- `cargo clippy -- -D warnings` passes.
- `cargo test -p scribefloat` passes.
- If you changed a `#[tauri::command]` signature, verify the JS caller in the corresponding panel HTML uses matching argument names (they are camelCase on the JS side).
- If you changed `Config`, verify a config file missing the new field still loads without panicking.
- If you changed anything in `platform/`, verify the other platform still compiles with `cargo check --target x86_64-pc-windows-msvc` (or equivalent).

---

## Unexplained numeric constants

Before changing a constant you did not write, run:

```bash
git log -S '<value>' -- <file>
git blame <file>
```

Find the commit that introduced it and read the message. Many audio and transcription constants were tuned empirically — the value often reflects a tradeoff, not an arbitrary choice.

---

## What is out of scope

Do not build these without explicit instruction:
- Linux support
- Mobile (iOS/Android)
- Cloud sync or accounts
- Pause/resume recording
- Webhook output
- Auto-escalation from Dictate to Scribe
