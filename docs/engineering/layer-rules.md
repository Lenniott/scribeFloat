# Layer Rules

> Load this when adding a feature, adding an IPC command, or deciding which layer a change belongs to.

---

## Call chain

```
panel (Svelte / TypeScript)
  → command (Tauri IPC — JS type → Rust type, one controller call, nothing else)
    → controller (owns Arc<Mutex<Inner>> state machine, orchestrates services)
      → service (stateless or singleton, created once in lib.rs::run())
        → platform (OS-specific, behind #[cfg(target_os)])
```

IPC: JS calls `invoke('command_name', { args })` → Rust `#[tauri::command]` in `commands/` → controller → service.

---

## Layer rules

**Commands** (`src-tauri/src/commands/`) — translate between JS types and Rust types and call one controller method. Nothing else. No business logic here.

**Controllers** (`src-tauri/src/controllers/`) — own the state machine (`Arc<Mutex<Inner>>`). Orchestrate calls to services. Do not open audio streams, write files, or check permissions directly.

**Services** (`src-tauri/src/services/`) — singletons created in `lib.rs::run()` and passed down. Never instantiated inside a controller.

**Platform** (`src-tauri/src/platform/`) — the only place `#[cfg(target_os = "...")]` is allowed. Everything above is platform-agnostic.

---

## Hard ownership rules

| Owner | What it owns |
|---|---|
| `HistoryService` | Structured record store (`{save_folder}/history.jsonl`): append, compact, tombstone delete. Every transcript-bearing flow always appends a record here on completion. |
| `OutputService` | Markdown rendering (pure free functions) and durable file I/O: `.md` writes (opt-in via `save_transcripts_as_markdown`), session manifests, post-transcription cleanup, dictate failure salvage, legacy reads, delete primitives. Dictate never writes `.md`. |
| `AudioService` | Opens audio streams and streams capture to checkpointed temp/session WAV files (16 kHz writer thread). Do not accumulate PCM in controllers. |
| `PermissionsService` | The only code that checks OS permissions. |

---

## Platform Adapter pattern

Any OS-specific behaviour lives behind a Platform Adapter. `#[cfg(target_os)]` checks belong only inside adapter implementations — never in controllers, services, or panels.

| Component | macOS | Windows |
|-----------|-------|---------|
| System audio capture | BlackHole via Core Audio | WASAPI loopback |
| Dictate paste | `enigo` via Accessibility API | `SendInput` |
| Permissions check | `AVCaptureDevice`, `AXIsProcessTrusted` | Registry (HKCU\...\Microphone) |
| Key listener | `CGEventTap` | win32 keyboard hook |
| Window activation | `setActivationPolicy` (NSApp) | n/a |

```rust
// Trait defined in platform/mod.rs
trait PlatformAdapter {
    fn capture_system_audio(&self) -> AudioStream;
}

#[cfg(target_os = "macos")]
struct MacOSAdapter;

#[cfg(target_os = "windows")]
struct WindowsAdapter;
```

---

## How to add a new IPC command

1. Add a `#[tauri::command]` fn to the relevant file in `commands/`.
2. Register it in the `tauri::generate_handler![]` macro in `lib.rs`.
3. If the command accepts user-supplied strings (paths, hotkeys, names), validate them in the command fn before passing to the controller. Reject early with a descriptive `Err(String)`.
4. Do not add logic to the command fn — call one controller method and return its result.

---

## How to add a new feature

1. Check `docs/action-flows.md` — if the behaviour is not described there, confirm scope before building.
2. Decide which layer it belongs to (controller, service, or platform adapter).
3. If it requires OS-specific behaviour, define a trait in `platform/mod.rs` and implement it per platform. The controller calls the trait, never the concrete type.
4. If it writes durable transcript **files** (`.md`, WAV cleanup, manifests), route through `OutputService`.
5. If it appends or updates the structured session **record** (`history.jsonl`), route through `HistoryService` (controllers orchestrate; never append JSONL from the frontend).
6. If it needs config, add a field to `Config` in `types.rs` with a `#[serde(default)]` so existing config files keep loading.
