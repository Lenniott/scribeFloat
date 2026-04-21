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

User records mic + system audio (remote call, meeting, etc).

1. User triggers Scribe via tray or hotkey
2. Scribe panel opens
3. Audio Service: Device Manager checks preferred mic → fallback if unavailable
4. Audio Service: checks system audio device available (BlackHole on macOS / WASAPI on Windows)
    - **Not available** → show setup guidance, speaker capture toggle disabled
    - **Available** → continue
5. User enables speaker capture toggle
6. Audio Service: Mic Capture opens mic input stream
7. Audio Service: System Audio Capture opens system audio stream, records speaker offset timestamp
8. Audio Service: Sleep Prevention acquired
9. Scribe panel enters **Recording** state — dual waveform active (mic + speaker channels)
10. User optionally types timestamped notes
11. User presses **Stop & Save**
12. Audio Service: both streams stopped, raw PCM buffers returned
13. Audio Service: Sleep Prevention released
14. Output Service: writes `mic.wav` and `speaker.wav` from buffers to session folder
15. Output Service: writes `session.json` — `{ speaker_offset_seconds, sample_rate }`
16. Check: is a model downloaded and selected?
    - **No model** → skip to step 24
    - **Model available** → continue
17. Scribe panel enters **Transcribing** state
18. Model Service: loads selected model(s)
19. Model Service: transcribes `mic.wav` → mic segments (progress 0–50%)
20. Model Service: transcribes `speaker.wav` → speaker segments (progress 50–100%)
21. Output Service: merges mic and speaker segments chronologically by timestamp
22. Output Service: suppresses near-duplicate lines from mic bleed
23. Output Service: applies `in:` / `out:` labels
24. Output Service: builds dual-source markdown transcript
25. Output Service: applies word replacement rules
26. Output Service: writes `<timestamp>_<model>.md` to save folder
27. Check: WAV retention setting
    - **Keep** → `mic.wav`, `speaker.wav`, `session.json` all kept
    - **Delete** → Output Service deletes all three only after transcript confirmed written and non-empty
28. Scribe panel enters **Done** state — file path(s) shown
29. **No model path**: all session files kept regardless of retention setting. Panel shows "Open in Transcribe →" with session path pre-filled

---

## 3. Dictate

Hotkey-driven. Always listening in background. No panel open beforehand.

### 3a. Double-tap mode

1. Hotkey Service detects first tap of configured key
2. Hotkey Service detects second tap within threshold window
3. Audio Service: Mic Capture opens mic input stream — audio buffered in memory only
4. Floating panel appears near cursor — does not steal focus
5. Panel shows waveform and elapsed timer
6. User speaks
7. Hotkey Service detects second double-tap (or user taps once to stop)
8. Audio Service: Mic Capture stops, returns raw PCM buffer from memory
9. Floating panel enters **Transcribing** state
10. Model Service: loads dictate model
11. Model Service: transcribes buffer → returns text
12. Output Service: applies word replacement rules (dictate scope)
13. Check: is there a focused text input?
    - **Yes** → paste text at cursor via OS input injection
    - **No** → copy text to clipboard + show system notification
14. Check: auto-enter setting on?
    - **Yes** → send Enter keystroke after paste
    - **No** → paste only
15. Output Service: appends to dictate log — `{ date, time, text }` → `dictate.jsonl` in transcripts folder
    - Empty transcript → skip log entry
16. Floating panel dismissed

### 3b. Hold mode

1. Hotkey Service detects key down
2. Audio Service: Mic Capture opens mic input stream — audio buffered in memory only
3. Floating panel appears near cursor
4. User holds key and speaks
5. Hotkey Service detects key up
6. Continue from step 8 above

---

## 4. Transcribe

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
| Scribe dual | Yes — `mic.wav` + `speaker.wav` + `session.json` | Output Service | Output Service | After transcript confirmed, if keep=off |
| Scribe no model | Yes — WAV only output | Output Service | Never | Always kept |
| Dictate | No — memory only | — | — | — |
| Transcribe | No — user owns source file | — | — | — |
