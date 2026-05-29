

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
