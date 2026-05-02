# scribefloat — Working Guide

> How to build, debug, and extend this codebase.
> For what the app does and how it is designed, read `docs/context.md` first.

---

## Read before touching anything

```
docs/context.md          ← Start here. Reading order for all other docs.
docs/architecture.md     ← Layer rules, call chain, C4 diagrams.
docs/folder-structure.md ← Where every file lives. Key rules for agents.
docs/requirements.md     ← Full feature spec. Source of truth for behaviour.
docs/action-flows.md     ← Step-by-step flows for each workflow.
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
- `OutputService` is the only code that writes to disk.
- `AudioService` is the only code that opens audio streams.
- `PermissionsService` is the only code that checks OS permissions.

---

## Scribe UI — recording auto-start (do not regress)

The Scribe webview is **prewarmed at startup** (`prewarm_scribe_window` in `src-tauri/src/lib.rs`). If the frontend defaults **`autoStart` / `autoStartRecording` to `true`**, the mic starts as soon as that hidden window loads — **not** when the user opens Scribe.

**Rules for agents:**

- **Never** default global Scribe auto-record to `true` in `src/routes/+page.svelte` to “fix” timing races.
- Recording should start only when the user **opens Scribe** (tray / hotkey → `open_scribe_window` emits `scribe://open-requested`; `+page.svelte` sets `autoStartRecording = true`) or taps **Start recording** / **Record again**.
- Use **`bind:autoStart`** on `ScribeScreen` and **`$bindable(false)`** in `scribe.svelte`; clear **`autoStart`** after a successful **`scribe_start`** so returning to **idle** does not immediately reopen the mic.
- Reopen/discard edge cases: rely on **`$effect`** + **`$bindable`** (and focus **`maybeAutoStartRecording`**), not **`autoStart = true` by default**.

---

## How to add a new IPC command

1. Add a `#[tauri::command]` fn to the relevant file in `commands/`.
2. Register it in the `tauri::generate_handler![]` macro in `lib.rs`.
3. If the command accepts user-supplied strings (paths, hotkeys, names), validate them in the command fn before passing to the controller. Reject early with a descriptive `Err(String)`.
4. Do not add logic to the command fn — call one controller method and return its result.

---

## How to add a new feature

1. Check `docs/requirements.md` — if it is not there, confirm scope before building.
2. Decide which layer it belongs to (controller, service, or platform adapter).
3. If it requires OS-specific behaviour, define a trait in `platform/mod.rs` and implement it per platform. The controller calls the trait, never the concrete type.
4. If it writes files, route through `OutputService`.
5. If it needs config, add a field to `Config` in `types.rs` with a `#[serde(default)]` so existing config files keep loading.

---

## How to investigate a bug

| Symptom | Start here |
|---------|-----------|
| Audio not capturing or wrong device | `services/audio.rs` → `MicSession` |
| Transcription wrong or failing | `services/model.rs` → `transcribe_pcm_with_progress` |
| Dual-source merge / mic bleed issue | `services/model.rs` → `merge_dual_source` |
| File not saved or wrong path | `services/output.rs` |
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
