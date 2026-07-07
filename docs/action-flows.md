# scribefloat — Action Flows

> Step-by-step flows for each workflow. These are implementation-agnostic.
> Use these as the source of truth for Level 3 architecture and agent implementation.

---

## 0. Onboarding

Triggered when `Config.onboarding_complete == false` at startup. Opens a dedicated 680×560 window (`?view=onboarding`). Runs once; can be restarted from Settings → Help.

This is the current designer-approved onboarding flow. It intentionally does not collect setup-personalisation answers; detailed configuration lives in Settings.

On mount, `onboarding.svelte` calls `model_list`. If any model is already downloaded, step 2 is skipped and the first downloaded model is auto-selected via `model_select` so Dictate has a model ready without the user re-visiting the download step.

**Step 1 — Welcome**
1. Onboarding window opens centered
2. Brand moment: app name, one-line description, feature pills
3. User chooses: "Get started" → step 2 (or 3 if skipped), or "Skip to Settings" → [OB-Exit-Settings]

**Step 2 — Model Download** _(skipped if a model is already installed)_
4. `model_list` loaded on mount; models displayed as table rows with size and download button
5. Progress tracked via `model://download-progress` events; polled every 2 s as fallback
6. Continue button appears once any model shows `downloaded = true`
7. Continue → `model_select` on first downloaded model (failure surfaces in UI, does not silently advance) → step 3
8. Skip button available if no download is in progress → step 3

**Step 3 — Permissions**
9. `settings_permissions_status` polled (every 5 s + on focus)
10. Microphone row: required; grant button calls `settings_permissions_request`
11. Accessibility row: optional; needed for Dictate auto-paste
12. Input Monitoring row (macOS only): optional; needed for key listener
13. Continue disabled until microphone permission is granted; optional permission state does not block progress → step 4

**Step 4 — Dictate Practice**
14. NoteComposer auto-focused on mount (correct target for `dictate_auto_paste` Cmd+V)
15. User double-taps modifier key, speaks, releases; transcribed text populates NoteComposer via `dictate://state-changed` DONE event
16. Live state indicator (pulsing dot) shown during RECORDING / TRANSCRIBING / PASTING
17. NoteComposer stays mounted (hidden via CSS) while active — preserves manual draft
18. ERROR state and empty-segment (TRANSCRIBING → IDLE) path both show inline hint
19. Auto-enter toggle: reads/writes `settings_get/set_dictate_auto_enter`; when on, DONE auto-submits note
20. Continue always available → step 5

**Step 5 — Feature Tour**
21. Stylised menu-bar graphic (live time, Wifi icon, app icon)
22. Four feature rows: Scribe, Transcribe, History, Settings
23. Platform-conditional login-item copy (macOS: System Settings → General → Login Items; Windows: Settings → Apps → Startup)
24. "Done" → [OB-Exit-Normal]

**Exit paths:**
- **OB-Exit-Normal**: `settings_complete_onboarding` → `getCurrentWindow().close()`
- **OB-Exit-Settings**: `settings_complete_onboarding` → `settings_show_window` → `getCurrentWindow().close()`

**Restart from Help:**
25. User clicks "Restart Setup Wizard" in Settings → Help
26. `settings_reset_onboarding` (sets `onboarding_complete = false`)
27. `settings_show_onboarding_window` → onboarding window opens at step 1

---

## 1. Scribe — Single Source

User records mic only. No system audio capture.

1. User triggers Scribe via **New note** tray item or hotkey (`CmdOrCtrl+Shift+L` default)
2. Scribe panel opens
3. Audio Service: Device Manager checks preferred mic → falls back to system default if unavailable
4. Audio Service: Mic Capture opens mic input stream
5. Audio Service: Sleep Prevention acquired
6. Scribe panel enters **Recording** state — waveform active, timer running
7. User optionally types timestamped notes during recording
8. User presses **Stop & Save**
9. Audio Service: Mic Capture finalizes `mic.wav` in the session staging folder (`{save_folder}/{timestamp}/mic.wav`)
10. ScribeController reads `mic.wav` into PCM for Whisper
11. Check: is a model downloaded and selected?
    - **No model** → skip to step 17
    - **Model available** → continue
12. Scribe panel enters **Transcribing** state — progress bar shown
13. Model Service: loads selected model (from cache or disk; tiny/base may already be preloaded)
14. Model Service: transcribes the mic PCM once → returns timestamped segments
15. ScribeController uses live voice-change cuts to build mic chunks
16. Speaker chunk service: embeds each chunk, groups chunk voiceprints into local speakers, derives transcript-level session speaker centroids from clean chunks, and maps Whisper segments to their parent chunk labels
17. Output Service: renders transcript markdown and applies word replacement rules
18. History Service: appends a JSONL record to `{save_folder}/history.jsonl` (always, regardless of markdown setting), including `speaker_change_cuts` and `speaker_chunks`
19. Check: `save_transcripts_as_markdown` setting
    - **On** → Output Service writes `{title}_{model}.md` to **save folder root** (appends `_1`, `_2`, … on collision); Done event carries `transcript_path`
    - **Off** → no `.md` written; Done event carries `transcript_path = None`
20. Check: WAV retention setting
    - **Keep** → staging folder and WAVs kept
    - **Delete** → Output Service removes staging folder after transcript confirmed non-empty
21. Scribe panel enters **Done** state — file path shown when available, Open Transcript button shown when path is present
22. **No model path**: staging WAV kept regardless of retention setting. Panel shows "Open in Transcribe →" with session path pre-filled

---

## 2. Scribe — Dual Source

User records mic + system audio (remote call, meeting, etc). Speaker capture can be toggled on/off at any point during the recording — the mic never stops.

1. User triggers Scribe via **New note** tray item or hotkey (`CmdOrCtrl+Shift+L` default)
2. Scribe panel opens; `captureSpeaker` initialised from the persistent settings default (off by default on fresh install)
3. Audio Service: Device Manager checks preferred mic → fallback if unavailable
4. Audio Service: Mic Capture opens mic input stream; Sleep Prevention acquired
5. Scribe panel enters **Recording** state — mic waveform active, timer running
6. User enables speaker capture toggle (can happen at any time during recording)
    - Platform Adapter: `loopback_device_and_config` finds the configured loopback device; if none configured, auto-detects any input device with "blackhole" in its name
    - Audio Service: output device switched to the preferred speaker route (e.g. "Liscribe" Multi-Output Device); previous output device saved for restore
    - Audio Service: System Audio Capture (loopback) stream opened; speaker waveform becomes active
    - ScribeController records `CapturedSpeakerSegment { start_ms, wav_path }` for this capture window (segment WAV streamed by Audio Service)
7. User may toggle speaker capture off during the recording
    - Audio Service: loopback stream stopped; segment saved to `SpeakerAccumulator`
    - Audio Service: output device restored immediately to previous value
    - User may re-enable again — each new segment is appended to the accumulator
8. **Toggle is session-only**: the in-recording toggle does NOT update the persistent settings default. Only the Settings page toggle changes the default for future sessions
9. User optionally types timestamped notes
10. User presses **Stop & Save**
11. Audio Service: mic and any active loopback streams finalized to disk; output device restored
12. ScribeController reads segment WAVs, assembles speaker PCM (`assemble_speaker_pcm`) — gaps between ON windows are silence
13. **RMS silence gate**: if assembled speaker PCM has RMS < −60 dBFS, speaker transcription is skipped; session treated as single-source
14. Output Service: writes merged `speaker.wav` to session folder when capture was active
15. Check: is a model downloaded and selected?
    - **No model** → skip to step 23
    - **Model available** → continue
16. Scribe panel enters **Transcribing** state
17. Model Service: loads selected model
18. Model Service: transcribes mic PCM → mic segments (progress 0–50%)
19. Model Service: transcribes speaker PCM → raw speaker segments (progress 50–100%) when dual-source
20. `filter_hallucination_phrases` (`services/output/hallucination.rs`): strips known Whisper hallucination phrases from mic and speaker segments (also applied in Dictate before `format_dictate_text` and Transcribe upload)
21. Model Service: merges mic and speaker segments chronologically; suppresses near-duplicate lines within 1.5 s; applies `in:`/`out:` labels
22. Output Service: groups segments, builds dual-source markdown, applies word replacement rules
23. History Service: appends a JSONL record to `{save_folder}/history.jsonl` (always, regardless of markdown setting)
    - Record fields: `speaker_capture` = persistent `scribe_capture_speaker` setting at write time; `dual_source` = speaker PCM was assembled and used for merge/transcription (false if capture was off, silence-gated, or no loopback audio)
24. Check: `save_transcripts_as_markdown` setting
    - **On** → Output Service writes `{title}_{model}.md` to save folder root (with `_1`, `_2`, … suffix on collision); Done event carries `transcript_path`
    - **Off** → no `.md` written; Done event carries `transcript_path = None`
25. Check: WAV retention setting
    - **Keep** → staging folder and WAVs kept
    - **Delete** → Output Service removes staging folder after transcript confirmed non-empty
26. Scribe panel enters **Done** state
27. **No model path**: staging files kept. Panel shows "Open in Transcribe →" with session path pre-filled

---

## 3. Dictate

Tray menu **Dictate** toggles the same pipeline as the key listener (default modifier is Left Control on macOS).

Key listener (always on): **Left Control** only (`CGEventTap` on macOS, low-level hook on Windows). Two sequences after an initial tap + release:

### 3a. Hold-to-talk (push-to-talk)

1. User taps Left Control, releases (short tap; long first press is ignored as a modifier chord)
2. User taps Left Control again within ~400 ms
3. Listener keeps second press in an **armed** state; mic stays closed until Left Control held ≥ ~500 ms (timer thread)
4. Once threshold crosses: Audio Service opens mic → floating panel opens near cursor → **RECORDING**
5. User speaks while Left Control stays down (releasing before RECORDING commits cancels the warm‑up HUD open)
6. User releases Left Control → mic stops → buffered PCM returned; continue with **Shared: after mic closes**

### 3b. Toggle mode

1. Steps 1–2 same as Hold-to-talk
2. Second Left Control tap is **released** before the ~500 ms hold threshold
3. On second release → mic opens (same RECORDING HUD)
4. User speaks hands-free after release
5. Third Left Control tap (after cooldown) stops capture
6. Then **Shared: after mic closes**

### Shared: after mic closes (either mode)

1. Audio Service: Mic Capture finalizes temp WAV under app local data (`dictate_temp/{uuid}.wav`)
2. ScribeController reads WAV → PCM for Whisper
3. Floating panel enters **Transcribing** state
4. Model Service: loads dictate model (tiny/base may already be preloaded)
5. Model Service: transcribes PCM → returns text
6. Output Service: applies word replacement rules (dictate scope)
7. Check: is there a focused text input?
    - **Yes** → paste text at cursor via OS input injection
    - **No** → copy text to clipboard + show system notification
8. Check: auto-enter setting on?
    - **Yes** → send Enter keystroke after paste
    - **No** → paste only
9. Audio Service temp WAV deleted on success; on failure Output Service may salvage to `{save_folder}/dictate_failures/`
10. History Service: appends a JSONL record to `{save_folder}/history.jsonl`
    - Empty transcript → skip log entry
    - Dictate never writes a `.md` file
11. On success the floating panel hides immediately (no lingering Done state). Paste failure shows a brief Done panel (~800 ms) with clipboard fallback text; errors auto-dismiss after ~800 ms. **X** during Transcribing or Pasting aborts the pipeline and returns to Idle.

---

## 4. Transcribe

User brings an existing audio file. No recording step.

1. User triggers Transcribe via the Upload area in the app shell
2. Transcribe panel opens
3. User selects audio file (WAV, MP3, M4A, FLAC)
4. User selects output folder (defaults to config save folder)
5. User selects model(s)
6. User presses **Transcribe**
7. Check: is selected file a dual-source session folder (contains `mic.wav` + `session.json`)?
    - **Yes** → dual-source flow (steps 8a–8d)
    - **No** → single-source flow (step 9)
8a. Model Service: transcribes `mic.wav` → mic segments (progress 0–50%)
8b. Model Service: transcribes `speaker.wav` → speaker segments (progress 50–100%)
8c. Output Service: merges, suppresses bleed, applies `in:`/`out:` labels
8d. Continue to step 10
9. Model Service: transcribes the decoded mic PCM once → returns timestamped segments
10. TranscribeController runs pitch/loudness analysis offline, builds chunks, embeds each chunk, and maps Whisper segments to their parent chunk labels
11. Output Service: renders markdown transcript and applies word replacement rules
12. History Service: appends a JSONL record to `{save_folder}/history.jsonl` (always, regardless of markdown setting), including `speaker_change_cuts` and `speaker_chunks` for single-audio uploads
13. Check: `save_transcripts_as_markdown` setting
    - **On** → Output Service writes `<source_filename>_<model>.md` to selected output folder; Done event carries `transcript_path`
    - **Off** → no `.md` written; Done event carries `transcript_path = None`
14. Transcribe panel enters **Done** state — file path shown when available, Open Transcript button shown when path is present

---

## 5. History view

Unified read-only and management view across all transcript-bearing flows.

### 5a. List

1. User opens the app via **Open scribefloat** tray item (navigates to Home) or uses in-app navigation
2. `history_list` IPC command → `HistoryController::list`
3. HistoryController reads all live records from `HistoryService` (last-writer-wins by id from `history.jsonl`; deleted tombstones excluded)
4. HistoryController reads legacy on-disk items: existing `.md` files via `OutputService::list_transcripts`, legacy dictate entries via `OutputService::read_dictate_history` (`dictate_history.json`)
5. Legacy items deduped: a legacy `.md` whose path matches a store record's `markdown_path` is suppressed (the store record takes precedence)
6. Merged list returned to frontend; legacy items carry prefixed ids (`md::` / `dictate::`) and are read-only

### 5b. Select and preview

1. User selects a history item from the list (title row or **View** icon on `NoteCard`)
2. List and filter tabs hide; **fullscreen detail** (`NoteDetailPane`) fills the window until Close
3. `history_get_detail` IPC command → HistoryController returns metadata for the selected record (`speaker_capture`, `dual_source`, etc.)
    - **Dual source** chip: `dual_source` true — merged speaker transcription ran
    - **Speaker capture** chip: `speaker_capture` true — setting was on when the record was written (may be true without dual source)
4. `history_render_markdown` IPC command → OutputService renders markdown from the record's segments (pure function, no disk read required unless already exported)
5. `NoteDetailPane` renders scrollable transcript preview, muted metadata chips, prev/next in the header, and item actions (Export / Open / Copy / Close) in `PanelFooter`
6. **Prev/next** (chevrons or Arrow keys) cycles within the active filter tab (All / Scribe / Dictate); changing tabs while detail is open closes detail if the item is not in the new filter

### 5c. Export to markdown (on demand)

1. User clicks **Export to Markdown** in `NoteDetailPane`
2. `history_export_markdown` IPC command → HistoryController
3. Output Service writes `.md` to save folder; HistoryService updates the record (`set_markdown_path`) — a new line for the same id is appended to `history.jsonl`
4. Detail pane updates to show the new file path and enables the **Open** button

### 5d. Open exported file

1. User clicks **Open** in `NoteDetailPane` (only shown when `markdown_path` is set)
2. OS opens the `.md` file in the configured or default viewer

### 5e. Delete

1. User clicks **Delete** on `NoteCard` (only available for store records, not legacy items); `history.svelte` opens a confirm modal
2. `history_delete` IPC command → HistoryController
3. HistoryService appends a tombstone (`deleted = true`) for the record id to `history.jsonl`
4. If the record has a `markdown_path`, Output Service deletes the `.md` file
5. If the record has a `session_dir`, Output Service recursively removes the kept audio directory (boundary-checked)
6. History list refreshes; deleted item is gone

### 5f. Read legacy items

- Legacy items (prefixed `md::` / `dictate::`) are read-only: they cannot be deleted via `history_delete` and cannot be exported via `history_export_markdown`
- `history_read_legacy` IPC command is available for direct legacy access if needed

---

## 6. Note editor (`/notes/[id]`)

Unified editor for store records (written, scribe, dictate with segments). Legacy `md::` / `dictate::` items stay on the list + `NoteDetailPane` read-only path.

### 6a. Create

1. User clicks **Record** in the TitleBar (or **+ New Note** on the Notes list) → `/notes/new` → `note_create_empty` appends one **written** line to `history.jsonl`
2. Redirect to `/notes/[id]`; if started from TitleBar **Record**, `note-editor` auto-starts capture via `appState.scribeAutoStart`

### 6b. Load

1. `history_get_detail` returns the record; `HistoryService` hydrates `.notes/{id}/written.md` and `meta.json` when present

### 6c. Autosave (editor)

1. CodeMirror debounce (~800 ms) → `note_save_written_content` only if body changed (dirty-check in UI)
2. Title debounce (~500 ms) → `note_save_title` only if title changed
3. Metadata sidebar → `note_set_tags` (and related metadata commands) on change
4. Backend overwrites sidecar files — **no new jsonl line**
5. Each metadata/content save emits `note://item-updated` with `{ id }`; `+layout.svelte` listens and calls `loadNotes()` so the notes list reflects title/tag changes without a full reload

### 6d. Attach transcript (recording strip)

1. TitleBar **Record** or `scribeController` → `scribe_*` commands → on completion `note_attach_transcript`
2. `update_segments` **appends** one jsonl line (capture event)

### 6e. Delete

1. Same as §5e; also removes `{save_folder}/.notes/{id}/`

### 6f. Leave guard (navigate away)

1. `note-editor.svelte` registers `appState.noteLeaveGuard`; `+layout.svelte` `beforeNavigate` and sidebar navigation call it when leaving `/notes/[id]`
2. **While recording** (`scribeController.phase === 'recording'`): navigate immediately — recording continues in background; note is not deleted
3. **`note_is_empty`** (no written body, no segments, default title unchanged) **and no metadata** → `history_delete` silently, then navigate
4. **Empty with metadata** (tags/keywords/layers in sidecar, no body or transcript) → “Discard empty note?” modal; Discard deletes, Keep cancels navigation
5. **Otherwise** → navigate; note persists

Logic lives in `src/lib/services/noteLeaveGuard.ts` (Vitest-covered).

---

## WAV lifecycle summary

Default save folder: `~/Documents/transcripts_scribefloat/` (configurable in Settings → General).

Voice learning controls live in Settings → Voice. `voice_learning_enabled` defaults off, voice embeddings are kept by default for current speaker matching, and `voice_embeddings_encryption_required` defaults on so automatic long-term learning can be blocked when encrypted storage is unavailable. If voice embedding retention is set to delete after transcript, Record and Upload keep transcript text, speaker labels, timings, chunk quality, and session speaker groups, but strip chunk and session speaker vectors before writing `history.jsonl`. If retention keeps vectors and the macOS Keychain-backed voice key is available, those vectors are encrypted at rest.

| Workflow | WAV written? | Who writes | Who deletes | When deleted |
|---|---|---|---|---|
| Scribe single | Yes — `{save_folder}/{timestamp}/mic.wav` streamed during capture | Audio Service (capture); Output Service (merged `speaker.wav` only in dual-source) | Output Service | Staging folder removed after successful transcript if keep=off |
| Scribe dual | Yes — `mic.wav` + per-segment `speaker_seg_*.wav`, merged `speaker.wav` | Audio Service (capture streams); Output Service (merged archive) | Output Service | Staging folder removed after successful transcript if keep=off |
| Scribe no model | Yes — staging WAV only | Audio Service | Never (until user deletes) | Always kept for Transcribe recovery |
| Dictate | Yes — temp `{app_data}/dictate_temp/{uuid}.wav` | Audio Service | Deleted on success; salvaged to `dictate_failures/` on error | After transcription completes or fails |
| Transcribe | No — user owns source file | — | — | — |

Transcripts (`.md`) are written to the **save folder root** as `{title}_{model}.md`, with `_1`, `_2`, … suffixes when the same title is reused.
