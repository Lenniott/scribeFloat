---
labels: [wayfinder:map]
---

# Map: Context chunking strategy (embedding/vectorization)

## Destination

Land a chunking strategy for `context_search.rs`'s embedding/vector pipeline that
groups transcript text by speaker turn, timestamp, and size in a way that (a)
produces coherent embedding vectors and (b) preserves enough line-level structure
(speaker + timestamp + verbatim text) to support a future retrieval pipeline:
query → vector match → LLM gate on the chunk's lines → LLM extracts the applicable
lines → reduced, cited context. Nothing here is built yet — this map exists to
carry the exploration forward across sessions.

A related but independently-shippable idea surfaced in the same exploration:
silence-triggered incremental Whisper transcription (chunk the audio itself at
silence boundaries during capture, transcribe each span as it completes, instead
of one full-buffer pass after stop).

## Notes

- Entry point / seam: `chunk_records()` in `src-tauri/src/services/context_search.rs`.
  Already a deep module (`chunk_records(save_folder, &[HistoryRecord]) ->
  Vec<ContextChunk>`) driving `build_index`/`search_index`/`export_context_pack`.
  Wired up today only via the `scribefloat-cli` binary (`index build` / `search` /
  `context`) — not yet an in-app IPC command.
- `SpeakerBlock` (identity/channel tier, `services/speaker_blocks.rs` +
  `services/speaker_align.rs`) is the existing speaker+text grouping the frontend
  (`TranscriptPanel.svelte`) already renders from. Chunking should reuse this same
  grouping as its outer loop rather than inventing a second one.
- Live PCM tap pattern (`Pcm16kTap` in `services/audio.rs`) is the proven plumbing
  for anything that needs to observe audio in real time during capture — live
  diarization (`LiveDiarization`) is the existing example to mirror for a silence
  segmenter.
- Whisper's Silero VAD (`services/model.rs`) only skips silence *inside* one
  full-buffer `whisper_full()` pass — it does not split capture into independent
  jobs. No live-during-recording silence-triggered transcription exists today.

## Decisions so far

- None binding yet — this is pre-ADR exploration. Candidate directions discussed:
  - Chunk boundary priority: speaker turn (primary) > silence gap (secondary,
    inside a same-speaker run) > size ceiling in target words + hard char cap
    (tertiary). Rationale: 2026-08-13 exploration session.
  - `ContextChunk` schema fork: split `text: String` into `embed_text: String`
    (what gets vectorized) + `lines: Vec<ChunkLine>` (speaker/timestamp/text per
    original segment, the retrieval payload for a future LLM gate/extract stage).
    Additive to the interface; `INDEX_SCHEMA_VERSION` bump + index rebuild.
  - Silence-triggered ASR chunking would reuse the `Pcm16kTap` pattern as a new
    consumer alongside live diarization; diarization alignment already works
    purely off absolute timestamps so it composes with segments arriving in waves.
    Open risk, not resolved: per-segment ASR passes lose cross-segment decoder
    context, may hurt accuracy right at silence-boundary sentences — needs a
    real accuracy check before committing.
- Found during this exploration, not part of the destination but worth acting on
  separately: `services/analysis.rs`'s `PitchAnalyzer`/`detect_cuts`/
  `SpeakerChangeCut` (ADR-0013) has zero downstream consumers today (frontend,
  chunking, and rendering all ignore it) — see `issues/01-remove-dead-pitch-loudness-analyzer.md`.

## Frontier

- `issues/01-remove-dead-pitch-loudness-analyzer.md` — open, ready-for-agent.
- No other tickets cut yet. Next session should turn "Decisions so far" into
  concrete tickets (chunk boundary policy, `ContextChunk` schema fork, silence-
  triggered ASR chunking) once the user is ready to move from exploring to
  building.

## Out of scope

- Anything past "vector match returns a chunk" — the LLM gate/extract stages are
  explicitly future work, only mentioned here as the reason `lines` needs to exist.
- Wiring `context_search.rs` up to an in-app IPC command (currently CLI-only) —
  not discussed, may be a separate decision.
