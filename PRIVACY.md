# Data Privacy & Security Policy — ScribeFloat

**Version 1.2 | Last updated: 2026-08-18**

This document is written for **security officers, IT auditors, and compliance teams** evaluating whether ScribeFloat can be deployed in their environment. It is intentionally comprehensive and direct.

---

## Quick-reference audit table

| Question | Answer |
|----------|--------|
| Does the app send audio to any server? | **No — never** |
| Does the app send transcripts to any server? | **No — never** |
| Does the app require an account or login? | **No** |
| Does the app collect telemetry or analytics? | **No** |
| Does the app phone home for updates? | **No — but "Check for updates" in Settings → Help makes a single opt-in request to `api.github.com` if the user clicks the button** |
| Does the app require an always-on internet connection? | **No** |
| Is an internet connection ever required? | **No — not for transcription. Optionally when the user manually clicks "Check for updates".** |
| Where is all user data stored? | **Exclusively on the user's local device** |
| What AI model runs the transcription? | **OpenAI Whisper Small (ggml format), bundled with the app and running locally** |
| What labels speakers in a recording? | **NVIDIA Sortformer v2 (ONNX), bundled; anonymous "Speaker N" slots the user can rename** |
| Does the AI model phone home? | **No — inference is fully offline; models are never downloaded at runtime** |
| What OS permissions are required? | **Microphone (all platforms), Accessibility (macOS only for paste), Input Monitoring (macOS only for hotkey)** |
| Can the app be used fully offline after install? | **Yes** |

---

## 1. Network activity

### 1.1 Outbound requests

Transcription models (Whisper Small, Silero VAD, and Sortformer ONNX) **ship inside the application bundle**. They are seeded into the local app-data `models/` directory on first launch. The app never downloads models at runtime — not on launch, not from Settings, and not from Hugging Face.

The only outbound network request the application can make is an **opt-in update check**:

| Attribute | Detail |
|-----------|--------|
| Destination | `https://api.github.com` |
| What is fetched | Latest release metadata: version string, release notes, release URL. No binary is downloaded automatically. |
| When it occurs | Only when the user explicitly clicks **"Check for updates"** in Settings → Help. Never automatically on launch or in the background. |
| What is sent | An HTTP GET request. `User-Agent: scribefloat/<version>` header. No audio, no transcripts, no personal data. |
| Protocol | HTTPS with certificate verification via `rustls-tls` |
| After fetch | If a newer version is found, a banner is shown with a "Open download page" button that opens the browser. Nothing is downloaded or installed automatically. |

### 1.2 No automatic network connections

The application makes **no automatic network connections** of any kind. There is no:

- Analytics or telemetry endpoint
- Crash reporting service
- Licence validation server
- Background update check (the update check in Settings → Help is manual and user-initiated only)
- Runtime model download (including Voice Activity Detection)
- WebSocket or long-lived connection
- DNS resolution for any domain other than (optionally) `api.github.com` on explicit user action

The Tauri WebView's Content Security Policy enforces this at the browser layer:

```
default-src 'self' asset: https://asset.localhost;
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' asset: https://asset.localhost data:;
connect-src 'self' ipc: http://ipc.localhost;
```

This CSP prevents the frontend from making any external HTTP requests — the only connect target is the local IPC bridge.

### 1.3 Network verification steps for auditors

To verify no unexpected outbound connections:

1. Run the app with a packet capture tool (e.g. Wireshark, Little Snitch, Windows Firewall log)
2. Use all features: open Record, record, Upload an audio file, use Dictate, open Settings — **without** clicking "Check for updates"
3. Confirm there are **no** DNS queries or TCP connections to external hosts (including Hugging Face)
4. Optionally click **"Check for updates"** once and confirm the only connection is to `api.github.com`

---

## 2. Audio handling

### 2.1 Microphone access

- Required for **Scribe** (recording sessions) and **Dictate** (voice-to-text)
- The OS presents a permission prompt on first use; the app does not bypass or pre-grant this
- Audio is captured in raw PCM format via [cpal](https://github.com/RustAudio/cpal), a cross-platform audio library
- Captured audio is streamed to 16 kHz WAV files on disk during recording via `AudioService` (a dedicated writer thread checkpoints headers every 30 s so a crash mid-session leaves a playable file). Transcripts, session manifests, and post-transcription cleanup are written by `OutputService`.

### 2.2 System audio (speaker capture)

- **Optional** — only active if the user explicitly enables the "Speaker capture" toggle in Scribe
- **macOS**: captured via [BlackHole](https://existential.audio/blackhole/) virtual audio device, which must be separately installed by the user. BlackHole is an open-source virtual audio driver; ScribeFloat does not bundle or install it automatically.
- **Windows**: captured via WASAPI loopback (built into Windows; no additional software required)
- No additional OS permission is needed beyond what BlackHole or WASAPI provides

### 2.3 Dictate audio — short-lived temp file

Dictate no longer holds the full recording in RAM for the duration of a session. While the hotkey is active, audio is streamed to a UUID-named temp WAV under the app local data directory (`dictate_temp/`). On successful transcription the temp file is deleted. If transcription fails, the WAV may be moved into `{save_folder}/dictate_failures/` for manual recovery — that salvage step is handled by `OutputService`.

Dictate does **not** append audio to `dictate.jsonl` or keep a standing audio archive. The temp file exists only for the active dictation (plus the optional failure salvage copy).

### 2.4 Scribe and Transcribe audio lifecycle

For Scribe recordings and the Transcribe file-import feature:

| Stage | Scribe (user records) | Transcribe (user drops file) |
|-------|-----------------------|------------------------------|
| Audio written to disk | Yes — `mic.wav` (and `speaker.wav` if dual-source) streamed during capture | No — user's source file is read but not copied |
| Written by | `AudioService` during capture; `OutputService` for merged speaker WAV, transcripts, manifests, cleanup | N/A |
| Deleted after transcription | Yes — session staging folder removed when **Keep audio after transcription** is off and the transcript is non-empty; WAVs kept in `{save_folder}/{timestamp}/` when that setting is on | N/A — user owns source file |
| Transcript location | Save folder root (`{title}_{model}.md`, with `_1`, `_2`, … suffixes on collision) | User-chosen output folder |
| Location while on disk | Staging WAVs in `{save_folder}/{timestamp}/`; transcripts at save folder root | N/A |

---

## 3. OS permissions

### 3.1 Permission table

| Permission | Platform | Purpose | Consequence of denial |
|------------|----------|---------|----------------------|
| **Microphone** | macOS + Windows | Capture audio for Scribe and Dictate | Scribe and Dictate cannot record; Transcribe (file import) still works |
| **Accessibility** | macOS only | Simulate Cmd+V to paste dictated text into the focused app | Dictate falls back to copying text to clipboard + showing a system notification; no paste injection |
| **Input Monitoring** | macOS only | Detect global keyboard events for Dictate hotkey | Dictate hotkey does not trigger; Scribe hotkey also affected |

### 3.2 How permissions are used

**Microphone**: The OS audio framework is queried to open a capture stream. The app calls `PermissionsService::statuses()` to check the current grant status and shows the result in Settings → Dependencies. It does not attempt to access the microphone unless the user initiates a recording action.

**Accessibility (macOS)**: Used exclusively to simulate a paste keystroke (`Cmd+V`) via the macOS Accessibility API after Dictate transcription completes. The app does **not** read any other application's UI elements, read clipboard contents of other apps, or observe any other accessibility data. The paste simulation is implemented in `platform/paste_impl.rs`.

**Input Monitoring (macOS)**: Used to register a `CGEventTap` that listens for modifier key state changes (key down / key up on the configured Dictate trigger key). The tap reads **raw keycode values only** — it does not perform string conversion, does not log keystrokes, and does not inspect non-modifier keys. The implementation is in `platform/key_listener.rs`.

### 3.3 Revoking permissions

All permissions can be revoked at any time via:
- **macOS**: System Settings → Privacy & Security → [Microphone | Accessibility | Input Monitoring]
- **Windows**: Settings → Privacy & Security → Microphone

Revoking a permission does not delete any existing data. The affected feature degrades gracefully (Scribe/Dictate cannot record; Dictate falls back to clipboard).

---

## 4. Local data storage

### 4.1 Storage locations

| Data | Location | Format |
|------|----------|--------|
| App configuration | OS app-data dir (`config.json`) | JSON |
| Whisper model files | OS app-data dir (`models/`) | Binary (ggml) |
| Sortformer diarization model | OS app-data dir (`models/`) | Binary (ONNX) |
| Transcripts | User save folder root (default: `~/Documents/transcripts_scribefloat/`) | Markdown (`.md`) |
| Audio recordings | User save folder, inside per-session subfolders (when retained) | WAV |
| Dictate history log | User save folder (`dictate.jsonl`) | JSONL |
| Dictate temp capture | App local data dir (`dictate_temp/`, deleted on success) | WAV |
| Dictate failure salvage | User save folder (`dictate_failures/`, only on transcription error) | WAV |

OS app-data directories:
- **macOS**: `~/Library/Application Support/com.benjamin.scribefloat-v8/`
- **Windows**: `%APPDATA%\com.benjamin.scribefloat-v8\`

### 4.2 Config file contents

`config.json` stores only user preferences and paths. It never contains audio data, transcript content, or credentials. Fields include:

- `save_folder` — path to transcript output directory
- `open_scribe_hotkey`, `dictate_hotkey` — hotkey strings (stored in config; not currently editable via the Settings UI)
- `include_timestamps` — whether transcripts include timestamps
- `keep_wav` — whether WAV recordings are retained after transcription (default: deleted)
- `scribe_capture_speaker` — whether speaker capture is enabled
- `preferred_input_device`, `preferred_speaker_device` — audio device names
- `input_label`, `output_label` — display labels for the two Scribe audio sources
- `theme_mode` — UI theme preference
- `open_with_app_path` — application used to open completed transcripts (macOS: app name; Windows: full exe path)
- `dictate_auto_paste`, `dictate_auto_enter` — Dictate behaviour flags
- `replacement_rules` — user-defined spoken-phrase-to-text substitution rules (e.g. "new line" → newline character); stored as a JSON array; no audio or transcript content
- `onboarding_complete` — first-run flag

### 4.3 Transcript file format

Each Scribe or Transcribe output is a single Markdown file named `<ISO-timestamp>_<model-id>.md`. The file contains timestamped transcription segments. For dual-source recordings, speaker lines are prefixed with `out:` and microphone lines with `in:`.

### 4.4 Dictate log format

`dictate.jsonl` appends one record per successful dictation:

```json
{"date": "2026-05-04", "time": "14:23:01", "text": "the transcribed text"}
```

Empty transcriptions are not logged. The file is never sent anywhere.

### 4.5 Atomic writes

Config updates use an atomic write pattern (write to a temp file, then `rename` to the final path). This prevents config corruption if the process is killed mid-write. Transcript files are verified non-empty before WAV files are deleted.

---

## 5. Third-party dependencies

All libraries run in-process with no network communication of their own.

| Library | Version | Purpose | Network access |
|---------|---------|---------|----------------|
| [whisper-rs](https://github.com/tazz4843/whisper-rs) | 0.13 | Whisper model inference (wraps whisper.cpp) | None |
| [cpal](https://github.com/RustAudio/cpal) | 0.15 | Cross-platform audio capture | None |
| [reqwest](https://github.com/seanmonstar/reqwest) | 0.12 | Update metadata fetch only | `api.github.com` when user manually checks for updates |
| [hound](https://github.com/ruud-v-a/hound) | 3.5 | WAV file read/write | None |
| [symphonia](https://github.com/pdeljanov/Symphonia) | 0.5 | Decode MP3, M4A, FLAC for Transcribe | None |
| [tauri](https://tauri.app) | 2 | App framework (Rust + OS WebView) | None at runtime |
| [enigo](https://github.com/enigo-rs/enigo) | 0.6 | Keyboard simulation for Dictate paste | None |
| [serde](https://serde.rs) + serde_json | 1 | JSON serialisation | None |
| [tokio](https://tokio.rs) | 1 | Async runtime | None |
| [chrono](https://github.com/chronotope/chrono) | 0.4 | Timestamp formatting | None |
| [uuid](https://github.com/uuid-rs/uuid) | 1 | Session ID generation | None |

No analytics SDKs, advertising SDKs, or crash reporting libraries are included.

---

## 6. AI model provenance

### Whisper transcription model

The app ships a single Whisper Small English ggml file (`ggml-small.en-q5_1.bin`). Upstream weights are published on Hugging Face for packaging; **the installed app never contacts Hugging Face**. The file is:

- Bundled in the release binary and seeded into the local app-data `models/` directory
- Verified with a pinned SHA-256 before inference; missing or corrupt copies fail with a clear offline/reinstall error
- A static inference artifact — it does not update itself or make network calls
- Run inside the app process via [whisper.cpp](https://github.com/ggerganov/whisper.cpp) (via the whisper-rs wrapper)

The model file does not contain user data. It is model weights produced by training on a public speech dataset.

### Silero VAD model

The Voice Activity Detection model (`ggml-silero-v6.2.0.bin`) is also bundled and seeded the same way. It is:

- A ggml-format version of [Silero VAD](https://github.com/snakers4/silero-vad), an open-source voice activity detector
- Used by whisper.cpp to skip silence between speech segments on clips long enough for VAD
- ~2 MB; required for normal-length transcription (missing/corrupt → offline/reinstall error; never re-fetched at runtime)
- Loaded and run entirely in-process alongside Whisper; makes no network calls

### Sortformer diarization model

The app ships NVIDIA Sortformer v2 (`diar_streaming_sortformer_4spk-v2.onnx`) for anonymous "who spoke when" labels inside a single recording. Upstream weights are published on Hugging Face for packaging; **the installed app never contacts Hugging Face**. The file is:

- Bundled in the release binary and seeded into the local app-data `models/` directory
- Verified with a pinned SHA-256 before inference; missing or corrupt copies degrade to an unlabeled transcript (diarization never fails a note)
- A static inference artifact — it does not update itself or make network calls
- Run in-process via `parakeet-rs`; controllers never touch ONNX directly (`services/diarization.rs`)

Sortformer does **not** identify people. It assigns anonymous slots (`Speaker 1`–`Speaker 4`) that the user can rename as plain text (`speaker_names.json`). No voice embeddings, enrollment clips, or biometric profiles are stored.

---

## 7. No automatic update mechanism

The application has no automatic update mechanism. It does not:

- Check for new versions on launch or in the background
- Download or execute update payloads automatically
- Schedule background processes for updates

Users can manually check for updates via **Settings → Help → "Check for updates"**. This button makes a single HTTP GET request to `api.github.com` to fetch the latest release metadata. If a newer version is available, the app shows a banner with release notes and a button to open the download page in the user's browser. No binary is downloaded or installed by the app itself.

---

## 8. Process isolation and sandboxing

ScribeFloat is a Tauri v2 application. The frontend (HTML/JS) runs in the OS WebView with a strict CSP. All sensitive operations (file I/O, audio, IPC, OS permissions) are handled exclusively in the Rust backend. The WebView cannot make file system calls or spawn processes.

The Tauri capability model (`src-tauri/capabilities/default.json`) restricts which IPC actions each window can invoke. The declared capabilities are:

```
core:default
core:window:allow-close
core:window:allow-hide
opener:default
dialog:default
clipboard-manager:allow-write-text
```

No capability grants filesystem read/write access directly to the WebView — all file operations go through validated Rust command handlers.

---

## 9. Compliance notes

### GDPR

ScribeFloat does not transmit personal data to any server. There is no controller-processor relationship with any third party for user data. All data is user-controlled and stored locally. If an organisation deploys ScribeFloat to process personal data in audio (e.g. meeting transcriptions containing names), the data-controller obligations rest with the organisation, not with the application vendor — the application itself never receives or processes that data centrally.

### HIPAA

Audio and transcripts remain local to the endpoint. The application imposes no access controls, encryption at rest, or audit logging beyond what the OS provides. Organisations transcribing protected health information (PHI) must ensure appropriate endpoint security controls (full-disk encryption, access control, audit logging) are in place on the device.

### SOC 2

There are no cloud components. The entire audit surface is the endpoint. There is no shared infrastructure, no multi-tenancy, and no vendor-operated data store.

### UK NCSC / ISO 27001

The application's local-only data processing model means it does not introduce a third-party cloud data-sharing risk. Bundled models involve no transmission of organisational data. The only optional external contact is the user-initiated update check to GitHub.

---

## 10. Full removal procedure

To completely remove all application data from a device:

1. Quit the application
2. Uninstall using the OS uninstaller
3. Delete the app data directory:
   - **macOS**: `rm -rf ~/Library/Application\ Support/com.benjamin.scribefloat-v8/`
   - **Windows**: `rmdir /s "%APPDATA%\com.benjamin.scribefloat-v8"`
4. Delete the transcript save folder (default):
   - **macOS**: `rm -rf ~/Documents/transcripts_scribefloat/`
   - **Windows**: `rmdir /s "%USERPROFILE%\Documents\transcripts_scribefloat"`

After these steps, no application data remains on the device.

---

## 11. Contact

For security-related questions or to report a vulnerability, open an issue on the project repository.
