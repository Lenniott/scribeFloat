---
title: "Triage: Opening transcript output allows any .md path"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Resolution

**Now, done 2026-07-29.** `TranscribeController::open_output_path` confines to `config.save_folder`, mirroring `SettingsController::open_transcript`'s existing pattern exactly (canonicalize both, require `starts_with`). Design call: confine to `save_folder`, not the per-request output folder — that folder isn't persisted anywhere retrievable at open time, so `save_folder` is the only base reliably available.

Follow-up found in the same session: the `save_transcripts_as_markdown` toggle was silently swapping a store-backed note's UI from the normal in-app editor to an external "Open .md" action once `has_markdown` flipped true — a genuinely different route, not just an auto-export side effect. Scoped that affordance (`NoteCard.svelte`, `NoteDetailPane.svelte`, `notes.svelte`) to `item.source !== 'store'` so legacy/orphaned `.md` items keep it (their only view) while store notes stay on their normal editor path regardless of the toggle.

Also surfaced and fixed a real, unrelated perf bug while testing this: `DiarizationService::ensure_model()` was re-hashing the full Sortformer ONNX file on every recording start (no cached fingerprint check, unlike Whisper's equivalent). Fixed in `7de6462`.

## Issue

`TranscribeController::open_output_path` can open any `.md` file anywhere on disk — it checks the extension but never confines the path to the configured save folder, unlike the sibling `SettingsController::open_transcript`, which already does exactly this confinement correctly. An inconsistency between two near-identical helpers, not a missing mechanism.

## Question

Read the "Opening transcript output allows any .md path" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now**, small — port `open_transcript`'s existing confinement pattern into `open_output_path`. Needs one small design call first: confine to `save_folder` or to whichever output folder was actually configured for that transcription run (they can differ)?

## Findings

- **Two separate "open output" commands exist with different confinement**:
  - `TranscribeController::open_output_path` (`src-tauri/src/controllers/transcribe.rs:171-190`, reached via `transcribe_open_output` in `src-tauri/src/commands/transcribe.rs:31-43`): canonicalizes the path, requires it to be a file, and requires the extension be `.md` (case-insensitive) — but does **not** confine it to the configured save folder. Any `.md` file anywhere on disk that the OS user can read can be handed to `open_file_for_user` (which shells out to `open`/`open -a <app>` on macOS, `cmd /c start` or a named exe on Windows — `src-tauri/src/platform/mod.rs:16-55`).
  - `SettingsController::open_transcript` (`src-tauri/src/controllers/settings.rs:241-256`): canonicalizes the path *and* canonicalizes `save_folder`, then rejects with `"transcript path is outside the configured save folder"` if the target isn't under the save folder (line 250). This is the safer helper referenced in the ticket.
- **Exposure**: Same threat model as ticket 18 — needs a compromised/malicious renderer to pass an arbitrary path into `transcribe_open_output`. Impact is bounded by the `.md` extension check (can't launch an executable/`.app` directly) and by `open_with_app_path` validation (`set_open_with_app_path`, settings.rs:220-239, requires absolute + existing path), but an attacker-controlled `.md` path outside the save folder could still be opened with the user's configured editor/app, e.g. to trigger app-specific parsing of an attacker-planted file, or as a disclosure oracle (success/failure reveals file existence at arbitrary paths).
- **Remediation (matches ticket ask)**: mirror `open_transcript`'s save-folder confinement inside `TranscribeController::open_output_path` — canonicalize `save_folder` from config and require `canonical.starts_with(&base)` before calling `open_file_for_user`, same as settings.rs:246-252. Note: `transcribe_open_output` may legitimately need to open files in a *different* output folder than `save_folder` per the `run_batch` comment (transcribe.rs:201-203, "`.md` is opt-in and may target a different output folder") — so confinement should be to whichever output folder was actually configured for that transcription, not hardcoded to `save_folder`; needs a small design decision, not just a copy-paste.
- **Size estimate**: Small — the confinement pattern already exists verbatim in settings.rs; porting it is a few lines, plus deciding which folder to confine to (save_folder vs. last-used output_folder) and a unit test analogous to `resolve_output_folder_rejects_relative_path` (transcribe.rs:550).
