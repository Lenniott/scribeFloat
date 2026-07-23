# Security review — `feature/0.3/embeds` spine

**Ticket:** [Security review with rubric](../issues/04-security-review-with-rubric.md)  
**Reviewed:** working tree on `feature/0.3/embeds` (HEAD `8843a2a` at review time)  
**Method:** written rubric → code evidence → severity guess → suggested bucket. Human sorts finally in a later ticket. No LGTM.

---

## Rubric

Score each area: **controls present?** / **trust boundary clear?** / **failure leaves sensitive data?** / **docs match code?**

| Area | What “good” looks like for this app |
|------|--------------------------------------|
| **IPC surface** | Commands take validated inputs; path args confined or dialog-sourced; destructive ops gated; window capability list matches real labels; XSS in any window cannot freely drive high-impact commands. |
| **Filesystem / note storage** | Save-folder confinement for reads that take paths; atomic writes for config/names; notes/history stay local; no accidental write outside configured roots. |
| **Secrets / keychain** | No API keys/credentials in `config.json`; legacy voice crypto key deleted when biometric stores are gone; missing key = success. |
| **Model files on disk** | Bundled/downloaded models verified (SHA-256) before inference; tampered files rejected; network fetches user-visible / opt-in where claimed. |
| **Paste / accessibility** | Accessibility used only for paste/Enter simulation; Input Monitoring listen-only for configured modifier; permissions documented and revocable; defaults don’t surprise. |
| **Legacy voice / biometric purge** | Startup purge imports names, deletes `voiceprints/` + `voiceprint_clips/`, deletes keychain key; history biometric fields dropped from on-disk JSONL (not only in memory). |
| **Dangerous shell / commands** | No user-controlled shell strings; `open` / helper binaries take fixed argv shapes; Windows `cmd` paths cannot inject. |
| **Dependency hotspots** | Native ML stacks (`whisper-rs`, ONNX/`ort` via `parakeet-rs`), input simulation (`enigo`, `rdev`/`CGEventTap`), network (`reqwest`), Tauri plugins scoped tightly. |

Severity guess scale: **Critical** / **High** / **Medium** / **Low** / **Info**.  
Bucket: **merge-blocker** (unease + real finding before merge) vs **Known issues** (park, do not redefine destination).

---

## Findings

### 1. Startup VAD download phones home without user action

- **Evidence:** `src-tauri/src/lib.rs` (~619–630) spawns `download_vad_model` when `vad_model_needs_redownload()`; `src-tauri/src/services/model.rs` `VAD_MODEL_URL` → `huggingface.co`. Contradicts `PRIVACY.md` §1.1 / §1.2 (“Never automatically on launch”, “no automatic network connections”).
- **Severity guess:** High (privacy-policy breach + unexpected outbound HTTPS on first runs / corrupt VAD).
- **Suggested bucket:** **merge-blocker** — either stop auto-download before merge, or rewrite PRIVACY and accept the behaviour explicitly.

### 2. Transcript HTML is unsanitized (`Options::all` + `{@html}`)

- **Evidence:** `HistoryController::render_transcript_html` uses `pulldown_cmark::Options::all()` then `html::push_html` (`src-tauri/src/controllers/history.rs` ~152–159). Frontend injects with `{@html html}` (`src/lib/ui/4_sections/TranscriptPanel.svelte` ~268). Speaker labels / segment text are user-influenced. CSP blocks many scripts (`tauri.conf.json` `script-src 'self'`) but does not neutralize HTML injection → event handlers / navigation / opener abuse; combined with flat IPC this is an escalation path.
- **Severity guess:** High (webview XSS → IPC under a process that already holds Mic / Accessibility / Input Monitoring).
- **Suggested bucket:** **merge-blocker** — sanitize or stop using `{@html}` / disable raw HTML in the markdown parser before merge confidence.

### 3. Legacy biometric vectors can remain on disk until compaction succeeds

- **Evidence:** Types drop embeddings on deserialize (`src-tauri/src/types.rs` `SpeakerChunk` / `SessionSpeaker` comments + test `legacy_history_line_with_embeddings_still_deserializes` ~1155–1166). On-disk rewrite is `HistoryService::compact` (`src-tauri/src/services/history.rs` ~277–302), spawned at startup with warn-and-skip on error (`src-tauri/src/lib.rs` ~765–768). Failed/skipped compaction leaves raw `embedding` / `encrypted_centroid_embedding` bytes in `history.jsonl`.
- **Severity guess:** High for users upgrading from voiceprint-era installs; Medium if no legacy data.
- **Suggested bucket:** **merge-blocker** if any real upgrade path retains old histories; else **Known issues** with an explicit “verify compaction ran” smoke item.

### 4. Keychain voice key deleted only when profiles dir is removed

- **Evidence:** Purge deletes key only when `report.profiles_dir_removed` (`src-tauri/src/lib.rs` ~700–703). `delete_voice_crypto_key` (`src-tauri/src/platform/mod.rs` ~150–174). If `voiceprints/` was already gone (manual delete, prior partial run) but the key remains, startup never calls delete. Clips-only leftover does not trigger key delete either.
- **Severity guess:** Medium as code hygiene; **blast radius is local only** — voiceprint never shipped (exploration / branch fog on the human’s machine; no released-user fleet). See map Decisions + ticket 14.
- **Suggested bucket:** **merge-blocker** (human: leave `main` as if voiceprint never happened) — not a multi-user upgrade scare.

### 5. Sortformer ONNX has no runtime integrity check

- **Evidence:** Whisper/VAD enforce SHA-256 (`src-tauri/src/services/model.rs` catalog + `get_or_load_context` / VAD verify). Sortformer filename constant only (`src-tauri/src/services/diarization.rs`); seed copy in `lib.rs` ~594–612 does not hash. Fetch script *does* pin checksum (`scripts/fetch-bundled-models.sh` line for `.onnx`), but a tampered `{app_data}/models/diar_streaming_sortformer_4spk-v2.onnx` still loads.
- **Severity guess:** Medium (local tamper → malicious ONNX / `ort` surface).
- **Suggested bucket:** **Known issues** (align with Whisper verify when capacity allows).

### 6. Flat IPC ACL: every capability window can invoke every command

> **Update 2026-07-19:** Ticket 16 closed (B+). Capabilities split per window; AppManifest lists commands; see `src-tauri/permissions/README.md`.

- **Evidence:** Single capability `src-tauri/capabilities/default.json` lists `dictate`, `history`, `onboarding` and grants plugin defaults; `invoke_handler` registers ~70+ commands with no per-window allowlist (`src-tauri/src/lib.rs` ~820–907). Dictate/onboarding webviews can call `history_delete`, `settings_set_open_with_app_path`, `transcribe_start`, etc. if JS is compromised.
- **Severity guess:** Medium (amplified by finding 2).
- **Suggested bucket:** **Known issues** (hardening); rises with XSS.

### 7. Transcribe path IPC reads arbitrary filesystem audio paths

- **Evidence:** `transcribe_inspect_inputs` / `transcribe_start` only reject empty strings (`src-tauri/src/commands/transcribe.rs` `validate_input_paths`). No save-folder confinement (unlike `history_read_legacy` / `settings_open_transcript`). By design for Upload, but any XSS can point at sensitive local files the process can read.
- **Severity guess:** Medium.
- **Suggested bucket:** **Known issues** (document as intentional; tighten if Upload paths become dialog-only tokens).

### 8. `transcribe_open_output` opens any `.md` file, not only save-folder

- **Evidence:** `TranscribeController::open_output_path` (`src-tauri/src/controllers/transcribe.rs` ~169–188) requires `.md` but does **not** call `within_save_folder`. Then `platform::open_file` + optional `open_with_app_path`. Contrast: `SettingsController::open_transcript` confines to save folder.
- **Severity guess:** Low–Medium.
- **Suggested bucket:** **Known issues**.

### 9. Broad Tauri plugin permissions (`opener:default`, `dialog:default`)

- **Evidence:** `capabilities/default.json`. Frontend `openUrl(updateResult.release_url)` (`src/lib/ui/5_views/setting_general.svelte` ~160) — URL comes from GitHub JSON via `UpdateService` (`src-tauri/src/services/update.rs`). `opener:default` is typically wide; no custom URL allowlist visible in-repo.
- **Severity guess:** Low–Medium (supply-chain / compromised release metadata → open attacker URL).
- **Suggested bucket:** **Known issues**.

### 10. Windows `open_file` uses `cmd /c start`; `open_with_app_path` is attacker-useful if IPC is owned

- **Evidence:** `src-tauri/src/platform/mod.rs` `open_file` — macOS `open [-a app] path`; Windows `Command::new(a).arg(path)` or `cmd /c start`. `set_open_with_app_path` accepts any existing absolute path (`src-tauri/src/controllers/settings.rs` ~220–238).
- **Severity guess:** Medium given IPC compromise; Low for honest UI use.
- **Suggested bucket:** **Known issues**.

### 11. Accessibility + Input Monitoring are powerful by design

- **Evidence:** Listen-only `CGEventTap` for Left Control (`src-tauri/src/platform/key_listener.rs`); paste/Enter via `enigo` (`paste_impl.rs`); `dictate_auto_paste` defaults **true** (`src-tauri/src/types.rs` Config). Entitlements only declare mic (`entitlements.plist`); Accessibility/Input Monitoring are TCC prompts (documented in `PRIVACY.md` §3).
- **Severity guess:** Info–Low as product necessity; residual risk if XSS (finding 2).
- **Suggested bucket:** **Known issues** (document; consider defaulting auto-paste off for stricter bar — product call).

### 12. Dependency / native-code hotspots

- **Evidence:** `src-tauri/Cargo.toml` — `whisper-rs` (+ Metal/OpenBLAS), `parakeet-rs`/`ort` (Sortformer ONNX), `enigo`, `rdev`, `reqwest`+rustls, `fastembed` linked into the lib (CLI `scribefloat` context index; not exposed as UI IPC today but increases binary/native surface). No automated advisory gate called out in this review.
- **Severity guess:** Medium (supply chain / memory-unsafety in native ML), ongoing.
- **Suggested bucket:** **Known issues**.

### 13. Legacy voiceprint purge exists and is mostly sound

- **Evidence:** `services/legacy_voice_purge.rs` (import names → `remove_dir_all` profiles/clips; unit tests). Wired at startup in `lib.rs` ~694–715. History biometric strip relies on serde ignore + compaction (see finding 3).
- **Severity guess:** Info (control present; residual in 3–4).
- **Suggested bucket:** n/a (positive control). Not a green light alone.

### 14. Path confinement on several read/open paths is real

- **Evidence:** `within_save_folder` (`controllers/history.rs` ~489–495); `read_legacy`, `open_transcript`, `ScribeController::read_transcript_at` canonicalize + `starts_with(save_folder)`.
- **Severity guess:** Info (good pattern — incomplete coverage, see 7–8).
- **Suggested bucket:** n/a.

---

## Rubric scorecard (quick)

| Area | Verdict |
|------|---------|
| IPC surface | Partial — confinement on some paths; flat ACL; HTML sink; arbitrary Transcribe paths |
| Filesystem / notes | Mostly OK for save-folder ops; Upload path IPC is wide |
| Secrets / keychain | No app secrets in config; legacy key delete incomplete (finding 4) |
| Model files | Whisper/VAD strong; Sortformer weak; VAD auto-fetch contradicts privacy doc |
| Paste / accessibility | Narrow use, listen-only tap; high privilege if webview compromised |
| Legacy biometric purge | Filesystem purge good; JSONL + keychain residuals remain |
| Dangerous shell | No free-form shell; fixed helpers; Windows `cmd` + open-with still sensitive |
| Dependencies | Concentrated risk in ML + input simulation stacks |

---

## Suggested human sort order (non-binding)

1. Finding 1 (VAD auto-network vs PRIVACY)  
2. Finding 2 (HTML / XSS → IPC)  
3. Finding 3 (biometric bytes until compact)  
4. Then park 4–12 into Known issues or follow-up hardening tickets  

Do **not** treat absence of Critical RCE as “safe to merge.” Human ticket sorts merge-blocker vs Known issues.
