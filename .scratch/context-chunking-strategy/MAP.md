---
labels: [wayfinder:map]
---

# Map: Context chunking strategy (embedding/vectorization)

## Destination

Land a chunking strategy for the CLI context index that treats the **Whisper line
as the only stored transcript**, stamps speaker on that line, derives UI turns
from consecutive same-speaker lines, and stores chunks as `note_id` + segment
indexes plus a binary vector. Speaker is a search filter, not an embed prefix.
Chunking runs only after Stop, when the Note is frozen — how ASR produced the
lines does not matter.

A future retrieval pipeline can hydrate those indexes, gate, and extract. LLM
stages and in-app search IPC are not this effort.

## Notes

- Seam: `chunk_records` / `build_index` / `search_index` (CLI `index build` /
  `search` / `context` today).
- Alignment already maps each Whisper line to a diarization (or channel) label,
  then copies the words into a parallel turn list. Tickets 02–05 stamp the label
  on the line and stop duplicating the words.
- Live PCM tap is the proven hook for a later silence segmenter (ticket 06);
  diarization already uses absolute timestamps, so waves compose. Indexing still
  waits until the Note is frozen.

## Decisions so far

- Binding: [ADR-0015](../../docs/adr/0015-whisper-line-is-the-transcript-atom.md)
  (2026-08-18). Ticket 01 closed.
  - Transcript atom = Whisper line with optional `speaker`. UI turns are a view.
  - Chunk = `{id, note_id, segment_indexes}` + binary vector. Do not persist the
    passage. Embed concatenated line text with no speaker names.
  - Homogeneous packing: speaker change first, then size ceiling. Silence / ASR
    job boundaries are **not** chunk cuts.
  - Speaker filter at search time, resolved from live line labels (relabel
    without rebuild still works).
  - Freeze-after-Stop: chunk + embed only when capture and speaker stamp are
    done. Incremental Whisper (ticket 06) may append waves; it must not change
    this schema.
  - Array index is the pointer because segments are append-only after freeze.
    Replace/splice ⇒ index rebuild. Stable ids only if we later edit in the
    middle.
- Ticket 02 closed (2026-08-18): alignment/channel labeling stamps
  `Segment.speaker`; `speaker_blocks` still written. Dictate / failed
  diarization leave speaker unset. Relabel still edits the turn list.
- Superseded candidate (2026-08-13): stored `embed_text` + `lines: Vec<ChunkLine>`
  copy of the transcript. Replaced by indexes into the Note (ADR-0015).
- Dead pitch/loudness analyzer (ADR-0013) removed — `issues/z_01-remove-dead-pitch-loudness-analyzer.md`.

## Frontier

Work the first open ticket whose blockers are closed:

1. [ADR — segment is the transcript atom](issues/01-adr-segment-is-the-transcript-atom.md) — **closed** (ADR-0015)
2. [Stamp speaker on each segment](issues/02-stamp-speaker-on-segment.md) — **closed**
3. [Transcript UI and relabel use segments](issues/03-derive-speaker-turns-from-segments.md) and [CLI index stores segment ranges not passage copies](issues/04-cli-index-chunks-as-segment-ranges.md) — **frontier**, parallel
4. [Stop persisting speaker_blocks](issues/05-stop-persisting-speaker-blocks.md) — after 03 and 04
5. [Silence-triggered Whisper (parked)](issues/06-silence-triggered-whisper.md) — **not** the frontier; needs an accuracy check first

## Out of scope

- LLM gate (“is this chunk relevant?”) and extract-indexes — future; see
  [docs/ideas/chunk.md](../../docs/ideas/chunk.md). Ticket 04 only has to return
  indexes so hydration works
- Wiring search up as an in-app IPC command (CLI-only stays)
- Treating silence as a chunk boundary
- Replacing the segment array at Stop without an index rebuild
