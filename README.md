# ScribeFloat

**Local-first AI transcription for macOS and Windows. No cloud. No accounts. No telemetry.**

ScribeFloat runs OpenAI's [Whisper](https://github.com/openai/whisper) model entirely on your machine. Audio is never sent to any server. Transcripts live on your local file system.

---

## Features

| Feature | Description |
|---------|-------------|
| **Scribe** | Record mic + optional system audio (meetings, calls). Transcribe to markdown when done. |
| **Dictate** | Hotkey-driven voice-to-text paste. Speaks into any focused input, anywhere on screen. |
| **Transcribe** | Drop an existing audio file (WAV, MP3, M4A, FLAC) and get a timestamped transcript. |
| **Dual-source** | Scribe can capture mic and speaker simultaneously, merging them into a labelled `in:`/`out:` transcript. |
| **Word replacement** | (Coming soon) Apply find/replace rules to every transcript automatically. |
| **Local models** | Choose from several Whisper model sizes. Downloaded once from Hugging Face, then run offline. |

---

## Platform support

| Platform | Status |
|----------|--------|
| macOS 13+ | Supported |
| Windows 10+ | Supported |
| Linux | Not supported |

---

## Privacy at a glance

All audio processing is local. The only outbound network request is a one-time Whisper model download from Hugging Face. Dictate audio is **never written to disk** — it lives in RAM for the duration of the recording and is discarded immediately after transcription.

See [PRIVACY.md](PRIVACY.md) for the full data-flow audit, OS permission breakdown, and compliance notes for security officers.

---

## Building from source

### Prerequisites

- **Rust** toolchain (1.70+) — [rustup.rs](https://rustup.rs)
- **Node.js** 18+
- **Tauri CLI v2** — `cargo install tauri-cli --version "^2"`
- **macOS only**: Xcode Command Line Tools, and [BlackHole](https://existential.audio/blackhole/) (for speaker capture)

### Commands

```bash
# Start dev build with hot reload
cargo tauri dev

# Production build
cargo tauri build

# Run unit tests
cargo test -p scribefloat

# Lint (must pass before committing)
cargo clippy -- -D warnings

# Fast type check without linking
cargo check
```

If `cargo tauri dev` fails with a missing asset error, verify that frontend HTML files under `src/ui/panels/` exist and that `npm install` has been run.

---

## First launch

The onboarding wizard runs on first launch and guides you through:

1. Granting **microphone** permission
2. Granting **Accessibility** permission (macOS — required for Dictate paste injection)
3. Setting up **system audio capture** (BlackHole on macOS / WASAPI on Windows)
4. Downloading a **Whisper model**
5. A **practice dictation** run

Once complete the app unlocks fully and the wizard can be replayed from **Settings → Help**.

---

## Hotkeys (defaults)

| Action | Default |
|--------|---------|
| Open Scribe | `Cmd/Ctrl + Shift + S` |
| Start / stop Dictate | Double-tap `Ctrl` (or hold `Ctrl` and release) |

Hotkeys are fully configurable in **Settings → Hotkeys**.

---

## Configuration

Config is stored as JSON in the OS app-data directory:

- **macOS**: `~/Library/Application Support/com.benjamin.scribefloat-v8/config.json`
- **Windows**: `%APPDATA%\com.benjamin.scribefloat-v8\config.json`

Transcripts and audio files are saved to `~/Documents/ScribeFloat/` by default. This can be changed in **Settings → General**.

---

## Architecture

See [context/architecture.md](context/architecture.md) for full C4 model diagrams.

The layered call chain:

```
panel (Svelte / TypeScript)
  → command  (Tauri IPC — type translation only, no logic)
    → controller  (owns state machine, orchestrates services)
      → service   (stateless singleton, created once in lib.rs)
        → platform  (OS-specific code only — behind #[cfg(target_os)])
```

Hard ownership rules:
- **`OutputService`** is the only code that writes to disk
- **`AudioService`** is the only code that opens audio streams
- **`PermissionsService`** is the only code that checks OS permission state

---

## Repository layout

```
src/                       SvelteKit frontend
  lib/
    components/            Reusable UI components (audio, form, layout, notes)
    screens/               Full panel screens (scribe, dictate, transcribe, settings)
    stores/                Svelte stores (model download state)
  routes/                  SvelteKit routes (+page.svelte, +layout.svelte)

src-tauri/                 Tauri / Rust backend
  src/
    commands/              Tauri IPC handlers (thin translation layer)
    controllers/           State machines and orchestration
    services/              Business logic singletons
    platform/              OS-specific implementations
    types.rs               Shared types, state enums, serialisation

context/                   Architecture and design documentation
  architecture.md          C4 model diagrams
  action-flows.md          Step-by-step workflow descriptions
  componets.md             UI component catalogue
  design-skill/            Design system tokens and query tool
```

---

## Releasing a new version

1. Bump the version in all three config files:
   ```bash
   npm run bump -- 0.2.0
   ```
2. Commit and tag:
   ```bash
   git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
   git commit -m "chore: bump version to 0.2.0"
   git tag v0.2.0
   git push origin main --tags
   ```
3. GitHub Actions picks up the tag and builds macOS (universal `.dmg`) and Windows (`.msi`) automatically. The release is published to GitHub Releases once both builds complete (~15–20 min).

The release body includes a note for macOS users to right-click → **Open** on first launch (the app is not notarized).

---

## Contributing

1. Read `context/architecture.md` before touching any Rust code
2. Run `cargo clippy -- -D warnings` and `cargo test -p scribefloat` before committing
3. If you add a new `#[tauri::command]`, register it in `lib.rs` and validate any user-supplied strings before passing to a controller
4. `#[cfg(target_os)]` checks belong only in `src-tauri/src/platform/` — never in commands, controllers, or services

---

## License

MIT — see [LICENSE](LICENSE).
