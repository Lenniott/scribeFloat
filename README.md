# ScribeFloat

**Local-first AI transcription. No cloud. No accounts. No telemetry.**

Primary platform: **macOS**. A **Windows** build is shipped, but it is not routinely tested on real hardware — see [Platform support](#platform-support) below.

ScribeFloat runs OpenAI's [Whisper](https://github.com/openai/whisper) model entirely on your machine. Audio is never sent to any server. Transcripts live on your local file system.

---

## Features

| Feature | Description |
|---------|-------------|
| **Scribe** | Record mic + optional system audio (meetings, calls). Transcribe to markdown when done. |
| **Dictate** | Hotkey-driven voice-to-text paste. Speaks into any focused input, anywhere on screen. |
| **Transcribe** | Drop an existing audio file (WAV, MP3, M4A, FLAC) and get a timestamped transcript. |
| **Dual-source** | Scribe can capture mic and speaker simultaneously, merging them into a labelled `in:`/`out:` transcript. |
| **Word replacement** | Apply find/replace rules to every transcript automatically. |
| **Local models** | Choose from several Whisper model sizes. Downloaded once from Hugging Face, then run offline. |

---

## Platform support

| Platform | Status |
|----------|--------|
| macOS 13+ | **Supported** — primary development target; releases are built, signed, notarized, and tested on real hardware |
| Windows 10+ | **Theoretically supported, untested** — the codebase has Windows implementations and CI publishes NSIS `.exe` installers, but day-to-day development and manual QA happen on macOS only |
| Linux | **Not supported** |

**Windows in practice:** Treat Windows as best-effort until more users validate it. If something breaks on your machine, please open an issue or PR — Windows contributors are especially welcome ([Contributing](#contributing)).

---

## Privacy at a glance

All audio processing is local. The only outbound network request is a one-time Whisper model download from Hugging Face. Dictate streams to a short-lived temp WAV during capture (deleted after success, or salvaged to `dictate_failures/` on error).

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
| Open Scribe | `Cmd/Ctrl + Shift + L` |
| Start / stop Dictate | Tap **Ctrl**, release; tap **Ctrl** again and hold ~0.5s to talk (release to finish), or tap–release twice quickly then **Ctrl** once to stop (toggle mode) |

Hotkeys are fully configurable in **Settings → Hotkeys**.

---

## Configuration

Config is stored as JSON in the OS app-data directory:

- **macOS**: `~/Library/Application Support/com.benjamin.scribefloat-v8/config.json`
- **Windows**: `%APPDATA%\com.benjamin.scribefloat-v8\config.json`

Transcripts and audio files are saved to `~/Documents/transcripts_scribefloat/` by default. This can be changed in **Settings → General**.

---

## Architecture

See [context/README.md](context/README.md) for the doc reading order and [context/architecture.md](context/architecture.md) for full C4 model diagrams.

The layered call chain:

```
panel (Svelte / TypeScript)
  → command  (Tauri IPC — type translation only, no logic)
    → controller  (owns state machine, orchestrates services)
      → service   (stateless singleton, created once in lib.rs)
        → platform  (OS-specific code only — behind #[cfg(target_os)])
```

Hard ownership rules:
- **`OutputService`** owns durable user-facing files (transcripts, manifests, cleanup, dictate history, failure salvage)
- **`AudioService`** opens audio streams and streams capture to checkpointed WAV files during recording
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
  components.md            UI component catalogue
  design-skill/            Design system tokens and query tool
```

---

## Releasing a new version

Releases are driven by **git tags** matching `v*.*.*` (for example `v0.2.12`). Pushing a tag starts the [Release workflow](.github/workflows/release.yml), which builds **three independent jobs**:

| Job | Runner | Artifact |
|-----|--------|----------|
| `build-macos-arm` | `macos-14` | Apple Silicon `.dmg` |
| `build-macos-intel` | `macos-15-intel` | Intel Mac `.dmg` |
| `build-windows` | `windows-latest` | Windows NSIS `.exe` |

Platform builds run in parallel. **`release` does not wait for Intel macOS** — it publishes as soon as Apple Silicon and Windows finish. The `attach-macos-intel` job adds the Intel `.dmg` later if that build succeeds.

### Prerequisites (one-time)

1. **Repository secrets** (Settings → Secrets and variables → Actions) for macOS signing and notarization:
   - `APPLE_CERTIFICATE` — base64-encoded `.p12` signing certificate
   - `APPLE_CERTIFICATE_PASSWORD`
   - `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` — notarization credentials

2. **`gh` CLI** (optional but handy): [cli.github.com](https://cli.github.com)

### Step 1 — Bump the version

From `main`, with a clean working tree:

```bash
npm run bump -- 0.2.12
```

This updates `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, then creates commit `chore: bump version to 0.2.12` and tag `v0.2.12`.

To bump and push in one step:

```bash
npm run bump -- 0.2.12 --push
```

To only edit the version files (no commit/tag):

```bash
npm run bump -- 0.2.12 --no-git
```

### Step 2 — Push to GitHub

If you did not use `--push`:

```bash
git push origin main
git push origin v0.2.12
```

Pushing the **tag** triggers CI. Pushing `main` alone does not.

### Step 3 — Watch the workflow

1. Open **GitHub → Actions → Release**.
2. You should see five jobs: three builds, `release`, and optionally `attach-macos-intel`.
3. Apple Silicon and Windows builds typically take **10–25 minutes** each. Intel macOS runs on `macos-15-intel` and does **not** delay the GitHub Release.
4. When Apple Silicon or Windows succeeds, `release` creates the GitHub Release. Intel attaches automatically via `attach-macos-intel` when its build completes.

Check status from the terminal:

```bash
gh run list --workflow=release.yml --limit 5
gh run watch   # follow the latest run
```

### Step 4 — Verify the release

1. Open **GitHub → Releases** and confirm tag `v0.2.12`.
2. Download the artifact for your platform:
   - **Apple Silicon Mac** → `.dmg` from the arm build
   - **Intel Mac** → `.dmg` from the intel build (not the arm build)
   - **Windows** → `_x64-setup.exe` installer

The release notes include a build-status table showing which platforms succeeded.

### Fixing a failed or missing platform build

You do **not** need a new version tag to retry a single platform.

1. Open the failed workflow run on Actions.
2. Click **Re-run failed jobs** (or re-run one job via ⋯ on that job).
3. After the build succeeds, either:
   - **Re-run the `release` job** from the same workflow run (⋯ → Re-run job) to attach the new artifact, or
   - Manually upload the installer to the existing GitHub Release.

If the `release` job was skipped on an older workflow run (before this split), open that run and **Re-run job → release** manually — it only needs the arm and Windows artifacts from that run.

If the `release` job never ran because both Apple Silicon and Windows failed, fix those builds first.

### Rebuilding the same tag (hotfix to CI or release config)

If you must ship a fix under the **same version** (for example a workflow-only change):

```bash
# After committing the fix on main:
git tag -d v0.2.12
git tag v0.2.12
git push origin :refs/tags/v0.2.12    # delete remote tag
git push origin main
git push origin v0.2.12               # re-triggers Release workflow
```

If a GitHub Release already exists for that tag, delete it first (Releases → ⋯ → Delete) or the upload step may conflict.

### Local release build (optional)

To build installers on your machine instead of CI:

```bash
npm ci
cargo tauri build                              # native macOS
cargo tauri build --target x86_64-apple-darwin # Intel slice from Apple Silicon host
cargo tauri build --target x86_64-pc-windows-msvc --bundles nsis  # Windows cross-build
```

macOS builds require Xcode command line tools; signed/notarized builds need local signing certificates equivalent to the CI secrets.

---

## Contributing

**Windows contributors are especially welcome.** Most day-to-day development happens on macOS, so we rely on Windows users to test releases, report bugs, and fix Windows-specific issues (permissions, audio devices, paste, installers, and anything under `src-tauri/src/platform/`). You do not need to own the whole app — reproducible bug reports, small fixes, and “this broke on my machine” PRs are all valuable.

1. Read `context/architecture.md` before touching any Rust code
2. Run `cargo clippy -- -D warnings` and `cargo test -p scribefloat` before committing
3. If you add a new `#[tauri::command]`, register it in `lib.rs` and validate any user-supplied strings before passing to a controller
4. `#[cfg(target_os)]` checks belong only in `src-tauri/src/platform/` — never in commands, controllers, or services

---

## License

MIT — see [LICENSE](LICENSE).
