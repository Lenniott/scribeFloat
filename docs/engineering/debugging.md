# Debugging Guide

> Load this when investigating a bug or performance issue.

---

## Where to start

| Symptom | Start here |
|---------|-----------|
| Audio not capturing or wrong device | `services/audio.rs` → `MicSession` |
| Transcription wrong or failing | `services/model.rs` → `transcribe_pcm_with_progress` |
| Dual-source merge / mic bleed issue | `services/model.rs` → `merge_dual_source` |
| Speaker channel hallucinating ("Thank you." etc.) | `controllers/scribe.rs` → `pcm_rms` (silence gate) and `filter_hallucination_phrases` |
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

The `on_tick` callback is called per Whisper segment. If progress appears stuck, the model is still running — Whisper does not yield between segments on a chunk.

---

## Unexplained numeric constants

Before changing a constant you did not write, run:

```bash
git log -S '<value>' -- <file>
git blame <file>
```

Find the commit that introduced it and read the message. Many audio and transcription constants were tuned empirically — the value often reflects a tradeoff, not an arbitrary choice.
