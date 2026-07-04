# Debugging Guide

> Load this when investigating a bug or performance issue.

---

## Where to start

| Symptom | Start here |
|---------|-----------|
| Audio not capturing or wrong device | `services/audio.rs` → `MicSession` |
| Transcription wrong or failing | `services/model.rs` → `transcribe_pcm_with_progress` |
| Dual-source merge / mic bleed issue | `services/model.rs` → `merge_dual_source` |
| Speaker channel hallucinating ("Thank you." etc.) | `services/output/hallucination.rs` → `speaker_pcm_has_signal` (silence gate) and `filter_hallucination_phrases` |
| Speaker capture not toggling or output device not restoring | `controllers/scribe.rs` → `toggle_speaker_capture`; check `restore_output_device` |
| Loopback device not found | `platform/mod.rs` → `loopback_device_and_config`; check BlackHole install or `preferred_speaker_device` config |
| Transcript paragraphs not grouping correctly | `services/output.rs` → `write_transcript`; check `MERGE_GAP_MS` and `speaker_source_prefix` |
| File not saved or wrong path | `services/output.rs` |
| History record missing or wrong after a session | `services/history.rs` → `append` / `compact` |
| History list wrong, merge/dedupe incorrect, or delete fails | `controllers/history.rs` |
| History detail layout, delete placement, or prev/next wrong | `docs/history-ui-review.md` → `transcripts.svelte`, `NoteDetailPane`, `NoteCard` |
| History chips wrong (dual source vs speaker capture) | `types.rs` `HistoryRecord::from_scribe` + `controllers/scribe.rs` write path |
| UI shows stale state | `commands/` fn for that panel → check emitted events |
| Hotkey not triggering | `lib.rs::run()` → `global_shortcut.on_shortcut` |
| Config not persisting | `services/config.rs` → `update()` and `save()` |
| macOS paste failing in Dictate | `platform/paste_impl.rs` |
| Permission check wrong | `platform/permissions_impl.rs` |

When a bug is in a tight loop (audio callback, transcription progress): add a `// BUG:` comment describing the issue, do not patch blindly. Audio callback code has timing constraints — understand the thread model before changing it.

---

## Debugging Whisper transcription

Whisper runs inside `tokio::task::spawn_blocking`. If you add logging or timing to the transcription path, use `eprintln!` or `std::time::Instant` — `tracing` spans do not propagate into blocking threads without extra setup.

The progress callback (`set_progress_callback_safe`) reports encoder/decoder work as 0–100% during `whisper_full`. Unlike `set_abort_callback_safe`, it is safe to register on whisper-rs 0.16 / Metal — only the abort callback triggers `GenericError(-6)`.

Scribe finalizes mic audio (`prepare_audio`) synchronously in `stop_and_save` before emitting `TRANSCRIBING`; only Whisper runs on the background blocking task.

Long recordings use whisper.cpp's internal seek/windowing — do not add arbitrary fixed-duration PCM windows in `services/model.rs`. Manual 10 s windows were a regression source.

**Speaker-aware chunking** — experimental and off by default via `Config.speaker_aware_chunking`: `services/speaker_chunking.rs` finds pitch (inline YIN)/loudness/silence handovers and `transcribe_pcm_with_speaker_cuts` runs Whisper per chunk. Cuts are logged with `tracing::info!` and stored on `SessionManifest.speaker_cuts` (Scribe). Not a substitute for voiceprint speaker ID (`speaker_blocks.rs`). Keep disabled unless actively benchmarking; the current Rust port does not yet match the validated recall target.

Benchmark (local fixture, not in git): `SPEAKER_CHUNKING_FIXTURE=~/Downloads/pitch_test/audio/test_audio.wav cargo test benchmark_recall_on_fixture -- --ignored` or `cargo run --features bench --bin speaker-chunk-bench -- <wav>`.

Silero VAD is disabled for clips under ~2 s (`VAD_MIN_PCM_SAMPLES`); shorter audio with VAD enabled often fails encode with `GenericError(-6)` because VAD strips all speech.

Record-start preload only warms the model file in the OS page cache (`warm_model_file_on_disk`) — it does not load a `WhisperContext` during capture, which would race with stop-and-transcribe on Metal.

Voiceprint enrollment uses live mic-level frame counts for clip purity — it does not call `transcribe_pcm_with_progress` (which previously shared Metal state with Scribe).

On GPU encode failure (Metal `GenericError(-6)` on M1), `transcribe_pcm_with_progress` retries on CPU, then without VAD if needed. The cached `WhisperContext` is reused across retries (fresh `WhisperState` per attempt); only `mark_cpu_fallback` evicts after a GPU encode failure. GPU is retried again on the next transcription.

An empty segment list after a successful `whisper_full` (whisper log: `single timestamp ending - skip entire chunk`) is normal silence/skipped-window behaviour — not an encode failure.

Do not wire `set_abort_callback_safe` on whisper-rs 0.16 / Metal — even when the flag is false, encode can fail with `GenericError(-6)`. Scribe used to pass an abort handle; Dictate never did. Cooperative cancel is checked between retry attempts instead.

### Known quirks (FYI for maintainers)

**Scribe encode failed with `GenericError(-6)` but Dictate worked** — Fixed by not registering Whisper's abort callback on Metal. Same audio, same model; only Scribe wired that callback.

**Cancel during transcription** — Cancel is checked between GPU → CPU → no-VAD retries, not mid-pass. The user may wait a few seconds for the current attempt to finish. Tradeoff for reliable encode on M1.

**Model reload each recording** — Each transcription clears the cached Whisper context so the next recording starts fresh on GPU. Slightly more load time after stop; avoids getting stuck after a failed attempt.

**Silence skipping (VAD) on short clips** — Disabled under ~2 s. Very short recordings transcribe without VAD so the encoder is not fed an empty buffer.

**Voiceprint enrollment** — Clip quality uses the live mic level meter, not a Whisper pass. Avoids fighting Scribe for the same GPU.

**Dual-source (mic + speaker)** — Mic and speaker each get their own VAD yes/no decision based on that track's length (`vad_path_for_pcm` per channel in `scribe.rs` and `transcribe.rs`).

**Regression test for a saved session WAV** — `transcribe_saved_scribe_mic_wav_matches_dictate_path` is `#[ignore]` and only runs when `SCRIBE_REGRESSION_WAV` points at a real `mic.wav` on the developer machine.

---

## Unexplained numeric constants

Before changing a constant you did not write, run:

```bash
git log -S '<value>' -- <file>
git blame <file>
```

Find the commit that introduced it and read the message. Many audio and transcription constants were tuned empirically — the value often reflects a tradeoff, not an arbitrary choice.
