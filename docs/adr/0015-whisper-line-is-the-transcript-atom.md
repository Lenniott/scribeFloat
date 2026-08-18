# ADR-0015: Whisper Line Is the Transcript Atom

**Status:** Binding
**Wayfinder:** Aspirational implementation — context-chunking-strategy (tickets 02–05). This document is binding; the code still stores a parallel turn list and a copied passage in the CLI index until those tickets land.

## Context

A Note’s transcript is produced by two clocks that never talk to each other: Whisper emits timed lines (`Segment`); Sortformer (or dual-source channel tags) emits timed speaker spans. Alignment joins them by maximum time overlap and today **copies the words** into `speaker_blocks` for the UI, leaving `segments` unlabeled. The CLI context index then copies those words again into `ContextChunk.text` and embeds that blob.

That duplication blocks the retrieval shape we want: query → optional speaker filter → vector match → hydrate lines from the Note → (later) LLM gate / extract by **index**. A stored passage cannot cite a Whisper line. A turn list that has already glued neighboring lines cannot give the LLM those indexes.

A related idea — silence-triggered Whisper during capture — must not force a second chunk schema. Chunking should not care how the lines arrived.

## Decision

**The Whisper line is the only stored transcript.** Each `Segment` carries optional `speaker` (plain text: `Speaker N`, a user-renamed name, or `In`/`Out` for channel-only captures). Alignment writes that field once. `segments` stay unlabeled when diarization is skipped or fails (Dictate, degrade path).

**UI turns are a view**, not a second transcript. Consecutive same-speaker lines group into the screenplay the panel already shows. Relabel writes `segment.speaker`. The stored `speaker_blocks` copy is expand-then-contract (keep writing in ticket 02, derive UI in 03, stop persisting in 05).

**A retrieval chunk is a pointer, not a copy:**

```
{ id, note_id, segment_indexes: [i, i+1, …] }  +  vectors.f32 row
```

The embedding input is concatenated line **text** with **no** speaker names, built at index time and discarded. Do not persist `embed_text` or a `lines` array. Do not put the float vector in JSON.

**Speaker is a filter, not an embed prefix.** Optional search filter: keep chunks whose live lines include that speaker (same pattern as today’s tag filter). Resolve labels from the Note at query time so a relabel does not require an index rebuild. Homogeneous packing (one speaker per chunk, then a size ceiling) makes that filter mean “this vector is that person’s words.”

**Freeze-after-Stop.** Chunking and embedding run only when capture is finished, speakers are stamped, and the Note’s `segments` array will not be rewritten. How ASR produced the lines (one full-buffer pass, silence-sized jobs, a future stitch) does not matter to the index.

### Locks (incremental ASR must not violate these)

1. **An ASR job is not a chunk.** Silence may split Whisper work; consecutive same-speaker lines still pack to the size ceiling.
2. **Segments are append-only after freeze.** Replacing or splicing the array requires an index rebuild.
3. **Index a Note only when capture + speaker stamp are done.** Do not grow a chunk’s index range while recording continues.
4. **Stamp speaker at Stop** (or one pass over all waves), even if text arrived earlier. Live diarization ranges are incomplete until flush.
5. **Array index is the pointer because of (2).** Stable `segment_id`s only if we later edit in the middle.

Whisper ↔ diarization **time overlap remains** the one-time stamp at write. Retrieval must not overlap clocks; it follows `segment_indexes`.

## Consequences

- **Easier:** one source of truth for words, times, and speaker. UI, markdown, relabel, and the index all read the same array. A search hit loads `history.jsonl` by `note_id` and picks lines by index.
- **Easier:** silence-triggered Whisper (parked) can append waves with absolute timestamps; the index schema does not change if freeze-after-Stop holds.
- **Easier:** BGE sees semantics only; “things Alice said” is a prefilter, not a token in the passage.
- **Harder / accepted:** until tickets 03–05 land, `speaker_blocks` still duplicates text (expand). Old notes without `speaker` on lines stay unlabeled and keep using the stored turn list as fallback.
- **Accepted:** mixed-speaker chunks are not the design. A size split may still yield several chunks from one long monologue, all the same speaker.
- **Accepted:** LLM gate/extract and in-app search IPC are out of this ADR. The index must return indexes so those stages can hydrate; they are not implemented here. The two-LLM sequence lives in [docs/ideas/chunk.md](../ideas/chunk.md).
- **Privacy:** no change to ADR-0014. Speaker values are plain names already on the Note. Vectors are local, derived from transcript text, never sent off-device.

## Ticket sequence

`.scratch/context-chunking-strategy/` — 02 stamp speaker (expand); 03 UI/relabel from lines; 04 CLI index as ranges; 05 stop persisting the turn list; 06 silence-triggered Whisper (parked, not frontier).
