# scribefloat — Action Flows

> Step-by-step flows for each workflow. These are implementation-agnostic.
> Use these as the source of truth for Level 3 architecture and agent implementation.

---

## 1. Scribe — Single Source

User records mic only. No system audio capture.

1. User triggers Scribe via tray or hotkey
2. Scribe panel opens
3. Audio Service: Device Manager checks preferred mic → falls back to system default if unavailable
4. Audio Service: Mic Capture opens mic input stream
5. Audio Service: Sleep Prevention acquired
6. Scribe panel enters **Recording** state — waveform active, timer running
7. User optionally types timestamped notes during recording
8. User presses **Stop & Save**
9. Audio Service: Mic Capture stops stream, returns raw PCM buffer
10. Audio Service: Sleep Prevention released
11. Output Service: writes `mic.wav` from buffer to session folder
12. Check: is a model downloaded and selected?
    - **No model** → skip to step 18
    - **Model available** → continue
13. Scribe panel enters **Transcribing** state — progress bar shown
14. Model Service: loads selected model(s)
15. Model Service: transcribes `mic.wav` → returns timestamped segments
16. Output Service: builds single-source markdown transcript
17. Output Service: applies word replacement rules
18. Output Service: writes `<timestamp>_<model>.md` to save folder
19. Check: WAV retention setting
    - **Keep** → WAV stays
    - **Delete** → Output Service deletes `mic.wav` only after transcript confirmed written and non-empty
20. Scribe panel enters **Done** state — file path(s) shown, Open Transcript button
21. **No model path**: Output Service keeps WAV regardless of retention setting. Panel shows "Open in Transcribe →" with audio path and save folder pre-filled

---

## 2. Scribe — Dual Source

User records mic + system audio (remote call, meeting, etc). Speaker capture can be toggled on/off at any point during the recording — the mic never stops.

1. User triggers Scribe via tray or hotkey
2. Scribe panel opens; `captureSpeaker` initialised from the persistent settings default (off by default on fresh install)
3. Audio Service: Device Manager checks preferred mic → fallback if unavailable
4. Audio Service: Mic Capture opens mic input stream; Sleep Prevention acquired
5. Scribe panel enters **Recording** state — mic waveform active, timer running
6. User enables speaker capture toggle (can happen at any time during recording)
    - Platform Adapter: `loopback_device_and_config` finds the configured loopback device; if none configured, auto-detects any input device with "blackhole" in its name
    - Audio Service: output device switched to the preferred speaker route (e.g. "Liscribe" Multi-Output Device); previous output device saved for restore
    - Audio Service: System Audio Capture (loopback) stream opened; speaker waveform becomes active
    - ScribeController records `SpeakerSegment { start_ms, raw_pcm, native_rate }` for this capture window
7. User may toggle speaker capture off during the recording
    - Audio Service: loopback stream stopped; segment saved to `SpeakerAccumulator`
    - Audio Service: output device restored immediately to previous value
    - User may re-enable again — each new segment is appended to the accumulator
8. **Toggle is session-only**: the in-recording toggle does NOT update the persistent settings default. Only the Settings page toggle changes the default for future sessions
9. User optionally types timestamped notes
10. User presses **Stop & Save**
11. Audio Service: mic stream stopped, raw PCM returned; any still-active loopback stream stopped and final segment saved
12. Audio Service: output device restored; Sleep Prevention released
13. `ScribeController.prepare_audio`: assembles all `SpeakerSegment` entries into one silence-padded 16 kHz PCM buffer (`assemble_speaker_pcm`) — gaps between ON windows are silence
14. **RMS silence gate**: if the assembled speaker PCM has RMS < −60 dBFS, speaker transcription is skipped entirely; session treated as single-source
15. Output Service: writes `mic.wav` (and `speaker.wav` if capture was active) to session folder
16. Check: is a model downloaded and selected?
    - **No model** → skip to step 24
    - **Model available** → continue
17. Scribe panel enters **Transcribing** state — indeterminate progress bar shown during model load
18. Model Service: loads selected model(s)
19. Model Service: transcribes `mic.wav` → mic segments (progress 0–50%)
20. Model Service: transcribes `speaker.wav` → raw speaker segments (progress 50–100%)
21. `filter_hallucination_phrases`: strips segments matching known Whisper hallucination phrases ("Thank you.", "Thanks for watching.", etc.) from speaker segments
22. Model Service: merges mic and speaker segments chronologically; suppresses near-duplicate lines within 1.5 s (mic bleed); applies `in:`/`out:` labels
23. Output Service: groups segments — same-source segments within 8 s merged into one paragraph; speaker-change boundaries use `\n`; same-source paragraph breaks use `\n\n`
24. Output Service: builds dual-source markdown transcript
25. Output Service: applies word replacement rules
26. Output Service: writes `<timestamp>_<model>.md` to save folder
27. Check: WAV retention setting
    - **Keep** → `mic.wav`, `speaker.wav` kept
    - **Delete** → Output Service deletes both only after transcript confirmed written and non-empty
28. Scribe panel enters **Done** state — file path(s) shown; `captureSpeaker` reset to the persistent settings default
29. **No model path**: all session files kept regardless of retention setting. Panel shows "Open in Transcribe →" with session path pre-filled

---

## 3. Dictate

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

1. Audio Service: Mic Capture stops, returns raw PCM buffer from memory
2. Floating panel enters **Transcribing** state
3. Model Service: loads dictate model
4. Model Service: transcribes buffer → returns text
5. Output Service: applies word replacement rules (dictate scope)
6. Check: is there a focused text input?
    - **Yes** → paste text at cursor via OS input injection
    - **No** → copy text to clipboard + show system notification
7. Check: auto-enter setting on?
    - **Yes** → send Enter keystroke after paste
    - **No** → paste only
8. Output Service: appends to dictate history (`dictate_history.json`)
    - Empty transcript → skip log entry
9. Floating panel dismissed (auto)

---


User brings an existing audio file. No recording step.

1. User triggers Transcribe via tray
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
9. Model Service: transcribes audio file → timestamped segments (progress 0–100%)
10. Output Service: builds markdown transcript
11. Output Service: applies word replacement rules
12. Output Service: writes `<source_filename>_<model>.md` to selected output folder
13. Transcribe panel enters **Done** state — file path shown, Open Transcript button

---

## WAV lifecycle summary

| Workflow | WAV written? | Who writes | Who deletes | When deleted |
|---|---|---|---|---|
| Scribe single | Yes — `mic.wav` | Output Service | Output Service | After transcript confirmed, if keep=off |
| Scribe dual | Yes — `mic.wav` + `speaker.wav` (speaker.wav written even if RMS gate skips transcription) | Output Service | Output Service | After transcript confirmed, if keep=off |
| Scribe no model | Yes — WAV only output | Output Service | Never | Always kept |
| Dictate | No — memory only | — | — | — |
| Transcribe | No — user owns source file | — | — | — |
