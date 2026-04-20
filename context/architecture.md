# Liscribe — Architecture

> C4 model: Context, Container, Component levels.
> All diagrams in Mermaid — agent and human readable.
> Platform adapter convention and permissions service defined at end.

---

## Level 1 — System Context

Who uses the system and what external systems it touches.

```mermaid
graph LR
    user["User\nMac or Windows\nUses Liscribe via GUI and hotkeys"]
    liscribe["Liscribe\nLocal-first desktop transcription\nNo cloud. No accounts."]
    hf["Hugging Face\nPublic model repository\nOne-time download, no account"]
    audio["OS Audio Layer\nmacOS: Core Audio + BlackHole\nWindows: WASAPI loopback"]
    clipboard["Clipboard and Input\nOS paste mechanism\nDictate output target"]
    fs["Local File System\nTranscripts, WAV files, config\nUser-chosen folders"]

    user -->|"records, transcribes, dictates"| liscribe
    liscribe -->|"downloads models once"| hf
    liscribe -->|"captures mic and system audio"| audio
    liscribe -->|"pastes dictated text"| clipboard
    liscribe -->|"saves transcripts and audio"| fs
```

---

## Level 2 — Containers

All internal containers and how they connect.

```mermaid
graph TB
    user["User"]
    tray["System Tray"]
    scribe["Scribe"]
    transcribe["Transcribe"]
    dictate["Dictate"]
    settings["Settings"]
    onboarding["Onboarding"]
    audio["Audio Service"]
    model["Model Service"]
    output["Output Service"]
    config["Config Service"]
    hotkey["Hotkey Service"]
    permissions["Permissions Service"]
    fs["Local File System"]
    hf["Hugging Face"]
    os_audio["OS Audio Layer"]
    clipboard["Clipboard and Input"]
    os_permissions["OS Permission APIs"]

    user --> tray
    tray --> scribe
    tray --> transcribe
    tray --> dictate
    tray --> settings
    tray --> onboarding

    scribe --> audio
    scribe --> model
    scribe --> output
    scribe --> config

    dictate --> audio
    dictate --> model
    dictate --> output
    dictate --> config
    dictate --> clipboard

    transcribe --> model
    transcribe --> output
    transcribe --> config

    settings --> config
    settings --> permissions
    onboarding --> config
    onboarding --> model
    onboarding --> permissions

    hotkey --> dictate
    hotkey --> scribe

    output --> fs
    output --> config
    model --> hf
    model --> fs
    audio --> os_audio
    dictate --> fs
    permissions --> os_permissions
```

---

## Level 3 — Components

### Audio Service

Capture only. Returns raw PCM buffers. Never writes to disk.

```mermaid
graph TB
    subgraph audio["Audio Service"]
        device["Device Manager"]
        mic["Mic Capture"]
        system["System Audio Capture"]
        session["Session Manager"]
        power["Sleep Prevention"]
        platform["Platform Adapter"]
    end

    config["Config Service"]
    os_audio["OS Audio Layer"]
    scribe["Scribe"]
    dictate["Dictate"]

    scribe -->|"start/stop recording"| session
    dictate -->|"start/stop recording"| mic
    session --> mic
    session --> system
    session --> power
    mic --> platform
    system --> platform
    device --> platform
    platform --> os_audio
    device --> config
```

**Component notes:**
- Device Manager: lists inputs, detects preferred mic, falls back to system default
- Mic Capture: opens mic stream, buffers raw PCM chunks
- System Audio Capture: macOS = BlackHole via Core Audio, Windows = WASAPI loopback
- Session Manager: coordinates both streams, tracks speaker offset, returns dual buffers
- Sleep Prevention: macOS = IOPMAssertionCreateWithName, Windows = SetThreadExecutionState. Acquired on stream open, released on close
- Platform Adapter: single point of OS divergence. Everything above is platform-agnostic

---

### Model Service

Download, load, transcribe, merge.

```mermaid
graph TB
    subgraph model["Model Service"]
        manager["Model Manager"]
        downloader["Downloader"]
        loader["Model Loader"]
        transcriber["Transcriber"]
        merger["Dual Source Merger"]
    end

    config["Config Service"]
    fs["Local File System"]
    hf["Hugging Face"]
    scribe["Scribe"]
    dictate["Dictate"]
    transcribe["Transcribe"]
    output["Output Service"]
    startup["App Startup"]

    startup -->|"load dictate + scribe default models"| loader
    scribe -->|"transcribe session"| transcriber
    scribe -->|"transcribe dual source"| merger
    dictate -->|"transcribe buffer"| transcriber
    transcribe -->|"load model then transcribe"| transcriber
    transcribe -->|"transcribe dual source"| merger
    merger --> transcriber
    transcriber --> loader
    loader --> fs
    manager --> fs
    manager --> config
    downloader --> hf
    downloader --> fs
    transcriber -->|"segments"| output
    merger -->|"merged segments"| output
```

**Component notes:**
- Model Manager: lists models, tracks downloaded status, reports file size
- Downloader: fetches `ggml-*.bin` from Hugging Face, streams to disk (progress UI TBD)
- Model Loader: loads ggml into memory, caches one `WhisperContext` per model id
- Transcriber: runs whisper-rs inference, returns timestamped segments, reports 0-100% progress
- Dual Source Merger: merges mic and speaker segments chronologically, suppresses mic bleed, applies in:/out: labels

**Loading strategy — no magic, no heuristics:**
- Dictate default model → loaded at app startup
- Scribe default model → loaded at app startup (likely same model as Dictate, shared cache)
- Transcribe default model → loaded when Transcribe action starts
- Extra models selected in Scribe panel → loaded when transcription starts after Stop and Save
- Loaded models cached for app session — no reload cost for consecutive uses

---

### Output Service

All file writes. Nothing else writes to disk.

```mermaid
graph TB
    subgraph output["Output Service"]
        wav_writer["WAV Writer"]
        formatter["Transcript Formatter"]
        replacements["Word Replacement"]
        file_writer["File Writer"]
        wav_cleanup["WAV Cleanup"]
        dictate_log["Dictate Log"]
    end

    config["Config Service"]
    fs["Local File System"]
    scribe["Scribe"]
    dictate["Dictate"]
    transcribe["Transcribe"]
    model["Model Service"]

    scribe -->|"PCM buffers"| wav_writer
    scribe -->|"save transcript"| formatter
    dictate -->|"save transcript + log"| formatter
    dictate -->|"append entry"| dictate_log
    transcribe -->|"save transcript"| formatter
    model -->|"segments"| formatter
    formatter --> replacements
    replacements --> file_writer
    file_writer --> fs
    file_writer -->|"confirmed written"| wav_cleanup
    wav_writer --> fs
    wav_cleanup --> fs
    dictate_log --> fs
    config -->|"replacement rules"| replacements
    config -->|"save folder"| file_writer
    config -->|"keep wav setting"| wav_cleanup
```

**Component notes:**
- WAV Writer: writes mic.wav, speaker.wav from PCM buffers, writes session.json
- Transcript Formatter: builds markdown from segments. Single source = timestamped lines. Dual source = in:/out: labelled lines merged chronologically
- Word Replacement: applies find/replace rules. Scope per rule: transcripts, dictate, or both
- File Writer: writes markdown to save folder, verifies file written and non-empty, sets permissions
- WAV Cleanup: deletes mic.wav, speaker.wav, session.json — only after transcript verified. Skipped if keep setting on. Skipped if no model was available at record time
- Dictate Log: appends to dictate.jsonl. Records date, time, text. Skips empty transcripts

---

### Scribe

Recording starts immediately on panel open. No explicit start button.

```mermaid
graph TB
    subgraph scribe["Scribe"]
        panel["Scribe Panel"]
        controller["Scribe Controller"]
        state["State Machine\nIDLE\nRECORDING\nTRANSCRIBING\nDONE\nNO MODEL"]
        waveform["Waveform Renderer"]
        notes["Notes Manager"]
    end

    audio["Audio Service"]
    model["Model Service"]
    output["Output Service"]
    config["Config Service"]
    hotkey["Hotkey Service"]
    transcribe["Transcribe"]

    hotkey -->|"open panel, recording starts immediately"| controller
    panel -->|"stop and save or cancel"| controller
    panel -->|"change mic, toggle speaker, select models"| controller
    controller --> state
    state -->|"recording starts on panel open"| audio
    state -->|"stop and save pressed"| model
    state -->|"save"| output
    state -->|"open in transcribe"| transcribe
    controller --> config
    panel --> waveform
    panel --> notes
    notes -->|"timestamps"| controller
    audio -->|"live PCM feed"| waveform
```

**State transitions:**
- IDLE → RECORDING: panel opens (hotkey or tray)
- RECORDING → TRANSCRIBING: Stop and Save pressed
- RECORDING → IDLE: Cancel pressed
- TRANSCRIBING → DONE: transcript written
- TRANSCRIBING → NO MODEL: no model downloaded at record time

---

### Transcribe

User brings existing audio. No recording step.

```mermaid
graph TB
    subgraph transcribe["Transcribe"]
        panel["Transcribe Panel"]
        controller["Transcribe Controller"]
        state["State Machine\nIDLE\nTRANSCRIBING\nDONE\nERROR"]
        file_picker["File Picker"]
    end

    model["Model Service"]
    output["Output Service"]
    config["Config Service"]

    panel -->|"file selected, model selected, transcribe pressed"| controller
    controller --> state
    controller --> file_picker
    state -->|"load model on action start"| model
    state -->|"transcribe file or session"| model
    state -->|"save transcript"| output
    controller --> config
    file_picker -->|"audio path"| controller
```

**Component notes:**
- File Picker: accepts WAV, MP3, M4A, FLAC. Detects dual-source session folder (contains mic.wav + session.json). Pre-fills path if opened via Open in Transcribe from Scribe

---

### Dictate

Always listening. Hotkey-driven. Audio in memory only — never written to disk.

```mermaid
graph TB
    subgraph dictate["Dictate"]
        controller["Dictate Controller"]
        state["State Machine\nIDLE\nRECORDING\nTRANSCRIBING\nPASTING"]
        hotkey_listener["Hotkey Listener"]
        float_panel["Floating Panel"]
        waveform["Waveform Renderer"]
        paste_handler["Paste Handler"]
    end

    audio["Audio Service"]
    model["Model Service"]
    output["Output Service"]
    config["Config Service"]
    clipboard["Clipboard and Input"]

    hotkey_listener -->|"start stop events"| controller
    controller --> state
    state -->|"open mic stream"| audio
    state -->|"show panel"| float_panel
    audio -->|"live PCM feed"| waveform
    float_panel --> waveform
    state -->|"transcribe buffer"| model
    model -->|"text"| output
    output -->|"formatted text"| paste_handler
    paste_handler --> clipboard
    controller --> config
```

**Component notes:**
- Hotkey Listener: detects double-tap and hold/release. Fires start/stop events to controller
- Floating Panel: appears near cursor, does not steal focus, shows waveform and timer only
- Paste Handler: uses Platform Adapter. macOS = Accessibility API paste. Windows = SendInput. Fallback = clipboard + system notification
- Audio buffer: memory only. No WAV written to disk under any circumstance

---

### Settings

```mermaid
graph TB
    subgraph settings["Settings"]
        panel["Settings Panel"]
        general["General Tab"]
        models["Models Tab"]
        hotkeys["Hotkeys Tab"]
        replacements["Replacements Tab"]
        audio_tab["Audio Tab"]
        deps["Dependencies Tab"]
        help["Help Tab"]
    end

    config["Config Service"]
    model["Model Service"]
    permissions["Permissions Service"]

    panel --> general
    panel --> models
    panel --> hotkeys
    panel --> replacements
    panel --> audio_tab
    panel --> deps
    panel --> help
    general --> config
    models --> config
    models --> model
    hotkeys --> config
    replacements --> config
    audio_tab --> config
    deps --> permissions
```

**Tab contents:**
- General: save folder, default mic, WAV retention, auto-enter, open transcripts with, start on login
- Models: per-action default model (Dictate / Scribe / Transcribe), download and delete models
- Hotkeys: Scribe hotkey, Dictate trigger key and mode. Save and restart to apply
- Replacements: add/edit/delete rules, scope per rule
- Audio: macOS = BlackHole status and setup guide. Windows = WASAPI device selector. Mic and speaker labels
- Dependencies: live permission status, one-tap path to OS settings pane
- Help: inline topics, no network required

---

### Onboarding

First-launch wizard. Blocks app until complete. Replayable from Settings → Help.

```mermaid
graph TB
    subgraph onboarding["Onboarding"]
        panel["Onboarding Panel"]
        step_mic["Step 1: Microphone"]
        step_accessibility["Step 2: Accessibility"]
        step_audio["Step 3: Audio Setup"]
        step_models["Step 4: Model Download"]
        step_practice["Step 5: Practice Run"]
        step_done["Step 6: Done"]
    end

    config["Config Service"]
    model["Model Service"]
    permissions["Permissions Service"]
    dictate["Dictate"]

    panel --> step_mic
    step_mic --> step_accessibility
    step_accessibility --> step_audio
    step_audio --> step_models
    step_models --> step_practice
    step_practice --> step_done
    step_mic --> permissions
    step_accessibility --> permissions
    step_models --> model
    step_practice --> dictate
    step_done --> config
```

**Step notes:**
- Step 1: request mic permission, verify granted before continuing
- Step 2: macOS = request Accessibility permission. Windows = skip
- Step 3: macOS = BlackHole install walkthrough. Windows = WASAPI confirmation
- Step 4: user picks model per action (Dictate / Scribe / Transcribe), download with progress
- Step 5: live Dictate practice test, user speaks and sees paste result
- Step 6: mark onboarding complete in config, app unlocked

---

## Architectural conventions

### Platform Adapter pattern

Any component with OS-specific behaviour isolates that behaviour behind a Platform Adapter.
Everything above the adapter is platform-agnostic. `#[cfg(target_os)]` checks belong only inside adapter implementations — never in controllers or panels.

Components requiring a Platform Adapter:
- Audio Service → system audio capture (BlackHole vs WASAPI)
- Dictate Paste Handler → paste mechanism (Accessibility API vs SendInput)
- Permissions Service → permission checks and OS settings deep-links

```rust
trait PlatformAdapter {
    fn capture_system_audio(&self) -> AudioStream;
}

#[cfg(target_os = "macos")]
struct MacOSAdapter;

#[cfg(target_os = "windows")]
struct WindowsAdapter;
```

### Call chain

```
panel (Svelte)
  → command (Tauri IPC — translation only)
    → controller (orchestration, owns state machine)
      → service (Audio, Model, Output, Config, Hotkey, Permissions)
        → engine (Recorder, Transcriber, Output builder — frozen once stable)
```

IPC: JS calls `invoke('command_name', { args })` → Rust `#[tauri::command]` in `commands/` → controller → service.

### Separation of concerns — hard rules

- Engine files are frozen once stable. Bugs get a `// BUG:` comment, not an in-place fix
- Services are singletons instantiated at app startup and passed down. Never instantiated inside controllers
- Controllers never import engine files directly — always through a service
- Output Service is the only component that writes to disk
- Audio Service is the only component that opens audio streams
- Permissions Service is the only component that checks OS permission state
