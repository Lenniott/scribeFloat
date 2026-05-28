# Remaining Work — Intel Mac Performance Branch

Items below are deferred from the initial performance pass (`claude/mac-intel-transcription-perf-TOUm4`).
Each section includes enough context to pick up the work independently.

---

## 1. Q4_0 Model Quantization

**Status:** Deferred. Current catalog still uses Q5_1 files.

**Why it matters:** Q4_0 decodes 3–5× faster on CPU with ≤0.1 absolute WER delta. The biggest
single-step improvement available without changing the inference engine.

**What to do:**

1. Update every entry in the model catalog (`src-tauri/src/services/model.rs`, the `catalog()` fn)
   to point at the `q4_0` variants on HuggingFace. File names follow the pattern:
   ```
   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-<name>-q4_0.bin
   ```
   Affected models: `tiny.en`, `base.en`, `small.en`, `medium.en`, `large-v3-turbo`.

2. Rename the catalog `id` fields from `*-q5` → `*-q4` (e.g. `"small-en-q5"` → `"small-en-q4"`).

3. Add a one-time config migration in `src-tauri/src/services/config.rs`:
   when the app loads and `selected_model_id` contains `-q5`, rewrite it to the corresponding
   `-q4` id and save. This prevents users with existing Q5 files from silently keeping a
   slow model selected.

4. Verify all Q4_0 file names exist on the HuggingFace repo before tagging a release — the
   `large-v3-turbo` variant name in particular has changed between whisper.cpp versions.

5. Update `PRELOAD_ELIGIBLE_MODEL_IDS` in `services/model.rs` to match the new ids
   (`"tiny-en-q4"`, `"base-en-q4"`).

**Files:** `src-tauri/src/services/model.rs`, `src-tauri/src/services/config.rs`,
`src-tauri/src/controllers/scribe.rs`, `src-tauri/src/controllers/dictate.rs`.

---

## 2. Homebrew-Free `SwitchAudioSource` Replacement

**Status:** Deferred. The Homebrew dependency remains in `platform/mod.rs`.

**Why it matters:** `restore_output_device` (called after Scribe finishes speaker capture to
put the system output device back) currently runs one of two Homebrew-installed binaries
(`SwitchAudioSource` or `switchaudio-osx`). On an Intel Mac without Homebrew — which is the
user profile most affected by the performance issues — both candidate paths fail silently,
leaving the system audio output stuck on BlackHole after every Scribe session.

**What to do:**

1. Write a minimal Swift helper (`src-tauri/Swift/SetDefaultOutput/main.swift`) that uses
   `kAudioHardwarePropertyDefaultOutputDevice` via CoreAudio to set the default output device
   by name. Command-line interface: `set-default-output "<device name>"`, exit 0 on success.

2. Add a build step to compile and sign the helper and copy it into the Tauri `.app` bundle
   (`Contents/MacOS/` or a `Contents/Helpers/` sub-directory). The Tauri build script
   (`src-tauri/build.rs`) is the right place to invoke `swiftc` on macOS.

3. In `src-tauri/src/platform/mod.rs`, replace the `which::which("SwitchAudioSource")` /
   `which::which("switchaudio-osx")` discovery block with a call to the bundled helper.
   Use `tauri::AppHandle::path().resource_dir()` to locate the helper inside the bundle.

4. Keep a Windows stub in `platform/mod.rs` that returns `Ok(())` so the project still
   compiles on Windows without changes.

5. Smoke-test on a fresh Intel Mac with no Homebrew: record a Scribe session with speaker
   capture enabled, stop, confirm the system output device returns to its original value.

**Files:** `src-tauri/src/platform/mod.rs`, `src-tauri/build.rs` (new Swift compile step),
`src-tauri/Swift/SetDefaultOutput/main.swift` (new file).

---

## 3. Low-RAM Warning in Model Picker

**Status:** Deferred. The `total_ram_bytes()` sysctl helper already exists in `lib.rs` but
is not yet exposed to the UI.

**Why it matters:** Selecting `medium.en` or `large-v3-turbo` on an 8 GB Intel Mac will cause
memory pressure that swaps out other processes. A soft warning in the model picker prevents
users from unknowingly choosing a model that will degrade their machine.

**What to do:**

1. Expose RAM to the frontend via a new Tauri command (e.g. `get_system_info`) that returns
   `{ total_ram_bytes: u64, physical_cores: u32 }`. Put the command in
   `src-tauri/src/commands/` following the existing pattern — no logic in the command fn.

2. In the model picker Svelte component (`src/lib/screens/setting_models.svelte`), call
   `get_system_info` on mount. If `total_ram_bytes < 8 * 1024^3`, render a warning adjacent
   to the `medium.en` and `large-v3-turbo` options. Pull the exact warning copy and styling
   from the design skill before writing any Tailwind classes:
   ```bash
   python3 context/design-skill/query.py search "warning"
   python3 context/design-skill/query.py ds get components.badge
   ```

3. The warning must be non-blocking (a badge/note beside the option, not a modal). The user
   can still select the model — this is advisory only.

4. `total_ram_bytes()` returns `None` on non-macOS; the command should handle this gracefully
   (return 0 or omit the field) so Windows doesn't show a spurious warning.

**Files:** `src-tauri/src/commands/` (new or extended command file), `src-tauri/src/lib.rs`
(register command), `src/lib/screens/setting_models.svelte`.

---

## 4. Two-Pass Draft + Refine Architecture (v2)

**Status:** Explicitly deferred. Do not start until v1 ships and RTF baselines exist from
real-user diagnostic logs.

**Why it matters:** Even with the v1 engine tuning, a single-pass transcription of a long
recording with a high-quality model has a 1–3 min wall time. A fast draft shown immediately,
then a background quality pass, makes the perceived latency near-zero.

**Shape of the work:**

### State machine changes

New states replacing `TRANSCRIBING → DONE`:
```
RECORDING → DRAFTING → REFINING → FINAL
```
- **DRAFTING**: fast model (tiny/base) produces a first transcript; shown to the user immediately.
- **REFINING**: quality model (small/medium) re-transcribes only segments where
  `avg_logprob < threshold` or `no_speech_prob > threshold`. Merged result is FINAL.
- DONE/ERROR remain as terminal error states.

### Confidence proxy

whisper-rs 0.16 exposes per-segment `avg_logprob` and `no_speech_prob` via `WhisperState`.
Spike the exact API surface before committing to a confidence formula — the segment iterator
API changed between 0.13 and 0.16.

Refine candidates: segments where `avg_logprob < -0.8` OR `no_speech_prob > 0.3` (starting
values — tune empirically once baselines exist).

### Settings UX

Replace the single model dropdown with two dropdowns:
- **Draft speed** — maps to a fast model (tiny/base). Default: `base.en`.
- **Final quality** — maps to a quality model (small/medium/large-v3-turbo). Default: `small.en`.

Predefined pairings to suggest:
| Label | Draft | Refine |
|---|---|---|
| Fast | tiny.en | base.en |
| Balanced | base.en | small.en |
| Quality | small.en | medium.en |

The existing model picker becomes a "Custom" fallback.

### Model eviction

v1 caches all contexts indefinitely. v2 needs to evict the draft model after FINAL to free
RAM before the refine model loads. Add an optional eviction path to `get_or_load_context`
keyed on a configurable `max_loaded_models: usize` (default 1 during the refine swap, 2 at
idle so both models stay warm for the next recording).

### Segment merge

After refine, re-merge: for each re-transcribed segment, replace the draft segment(s) covering
the same time range. Keep draft segments for anything the refiner did not touch.

**Files (when ready to start):** `src-tauri/src/controllers/scribe.rs` (state machine),
`src-tauri/src/services/model.rs` (eviction, per-segment confidence), `src-tauri/src/types.rs`
(new `Segment` confidence fields), relevant Settings Svelte components.

---

## Verification Checklist (all items)

- [ ] Q4_0 files confirmed present on HuggingFace before tagging
- [ ] Config migration: app with `small-en-q5` selected starts, migrates to `small-en-q4`, triggers download
- [ ] Homebrew-free test: fresh Intel Mac, no Homebrew — speaker capture restores output device
- [ ] Low-RAM warning renders on a machine reporting < 8 GB; absent on a machine with ≥ 8 GB
- [ ] Low-RAM warning absent on Windows (no sysctl available)
- [ ] Two-pass: draft transcript visible within ~5 s of stop on `base.en`; refine completes in background
- [ ] Two-pass: final transcript replaces draft without flicker or scroll-position reset
- [ ] `cargo clippy -- -D warnings` clean after each item
- [ ] `cargo test -p scribefloat` passes after each item
