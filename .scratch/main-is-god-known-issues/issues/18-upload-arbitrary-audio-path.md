---
title: "Triage: Upload can read any audio path you pass it"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Upload can read any audio path you pass it" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- **Current behavior**: `transcribe_inspect_inputs` and `transcribe_start` (`src-tauri/src/commands/transcribe.rs:6-29`) accept `input_paths: Vec<String>` from the renderer and only reject empty/blank strings (`validate_input_paths`, line 52). No extension allowlist, no directory confinement, no canonicalization check before use — any absolute or relative path the frontend supplies is passed straight to `TranscribeController::inspect_inputs` / `::start`, which ultimately calls `decode_input` (`src-tauri/src/services/transcribe_input.rs:105`) to read and decode the file as audio.
- **Exposure**: Requires renderer→Rust IPC compromise (XSS in the webview, or a malicious dev tool) since these are plain `#[tauri::command]`s reachable from JS with no capability restriction beyond the `shell` window's `main-shell` permission set (`src-tauri/capabilities/shell.json`). A compromised renderer could pass any file path readable by the OS user (not limited to audio) and have it decoded/processed — best case it errors as non-audio, worst case it's a disclosure oracle (success/failure or transcribed content reveals file existence/contents for text-like files fed through the audio decoder).
- **Contrast with tickets 19/21 fix pattern**: settings.rs `open_transcript` (ticket 19's "good" example) canonicalizes and confines to `save_folder`. Upload has no equivalent concept of a confinement root since by design it must read arbitrary user-picked files (that's the "pick a file" UX) — so a folder allowlist doesn't fit; the realistic mitigation is scoping via the `dialog` plugin's file picker (Tauri fs scope / `dialog:allow-open` capability) so paths only ever originate from an OS native file picker rather than arbitrary renderer-supplied strings, i.e. the "Later: Dialog-only tokens" note in the map already identifies the correct fix.
- **Remediation if pursued**: enforce that `input_paths` come from a Tauri dialog scope (capability-restricted `fs` scope tied to the dialog plugin) instead of raw strings from JS; alternatively add an extension allowlist (audio/video mime-adjacent extensions) as defense-in-depth even though it doesn't fully close the path-traversal-style read.
- **Size estimate**: Medium — requires reworking the Upload IPC surface to use dialog-scoped tokens/paths rather than a `Vec<String>` argument, plus frontend changes to how Upload passes selections to `transcribe_start`. Not a one-line fix; matches the map's existing "out of scope this map" / Later classification.
