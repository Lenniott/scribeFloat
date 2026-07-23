---
title: Bundle-only models — no runtime downloads
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Can the app run with **only** models shipped in the bundle — no Hugging Face (or other) downloads at runtime — and do PRIVACY / README / site / CONTEXT tell that same story?

**Done when:** Startup no longer phones home for VAD or any model; missing/corrupt model → clear offline error; public docs match (no Settings → Models download story; no “download Whisper sizes”).

## Spec (to-spec)

Evidence: security S1 (`lib.rs` startup → `download_vad_model` → `huggingface.co`); architecture F2 (README / PRIVACY / `site_content.json` / CONTEXT still sell Settings → Models).

**In this ticket:** kill runtime model fetch + rewrite public honesty docs.  
**Not this ticket:** Sortformer SHA before load → *Verify all bundled models* (15). ADR fast/refined wording → *Mark and amend ADRs* (18). Record/Dictate rename unify → (17). Keep opt-in `api.github.com` update check (already user-initiated; stays in PRIVACY).

### Code cut (fixed)

1. Remove startup spawn in `lib.rs` that calls `download_vad_model`.
2. Delete `download_vad_model`, `VAD_MODEL_URL`, and related retry/progress emit path in `services/model.rs`.
3. Delete `ModelDownloadEvent` + `model://download-progress` (no frontend listeners; only VAD fetch emitted it).
4. Rename `vad_model_needs_redownload` → integrity/availability helpers only (no “redownload” concept).
5. Whisper missing/corrupt already says reinstall (ticket 08) — keep that; do not point at Settings → Models or Hugging Face.
6. `reqwest` remains for update-check only.
7. Dev/release still seed from bundle resources via `scripts/fetch-bundled-models.sh` (build-time / packaging — not runtime).

### Docs cut (fixed)

| File | Change |
|---|---|
| `PRIVACY.md` | No Hugging Face / Settings → Models story. Internet never required for models. Only optional update check. Auditor steps drop HF. |
| `README.md` | Bundled Whisper Small (+ VAD / voiceprint as shipped). Privacy glance: no model download. |
| `site_content.json` | Setup step 02 + privacy pillar “One network call” rewritten for bundle-only. |
| `CONTEXT.md` | Settings no longer “models”; Record/Dictate no longer imply separate fast/refined downloadable tiers. |

### Aggression (agreed)

**(2) Hard VAD** — missing/corrupt VAD fails with the same reinstall/offline story as Whisper; do not silently skip.

### Done when

1. Grep: no `huggingface.co` / `download_vad_model` / `VAD_MODEL_URL` under `src-tauri/src/`.
2. Startup never opens a network connection for models (update check unchanged).
3. Public docs above tell one story: models ship in the app; no Settings → Models; no “download Whisper sizes.”
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass.

## Resolution

Hard VAD (option 2) implemented on `feature/0.3/embeds` working tree.

| Cut | Result |
|---|---|
| Startup VAD Hugging Face fetch | Removed from `lib.rs`; warn-only if bundled VAD missing/corrupt |
| `download_vad_model` / `VAD_MODEL_URL` / `ModelDownloadEvent` | Deleted |
| `vad_path_for_pcm` | Returns `Result`; short clips skip VAD; long clips hard-fail with reinstall copy |
| PRIVACY / README / `site_content.json` / CONTEXT | Bundle-only story; no Settings → Models; no runtime HF |

**Verify:** `cargo test -p ScribeFloat --lib` → 333 passed; `cargo clippy -p ScribeFloat -- -D warnings` clean. Grep: no `huggingface.co` / `download_vad_model` under `src-tauri/src/`.

## Comments

- 2026-07-19: claimed; to-spec drafted; human chose hard VAD (2); implemented and closed.
