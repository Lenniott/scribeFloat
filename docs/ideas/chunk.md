# Retrieval after chunking (gate + extract indexes)

Parked from [context-chunking-strategy](../../.scratch/context-chunking-strategy/MAP.md). The effort **stops after the index** (chunk = `note_id` + segment indexes + a vector). This note is the future pipeline those indexes exist for.

Binding model: [ADR-0015](../adr/0015-whisper-line-is-the-transcript-atom.md). Do not relitigate speaker-on-the-line, freeze-after-Stop, or “don’t store the passage.”

## Summary

Local query → optional speaker filter → vector match → hydrate Whisper lines from the Note → LLM gate (relevant?) → LLM returns **segment indexes** → cited lines only.

Vectors never copy the transcript. The Note’s `segments` array is the text. Speaker names are a **filter**, not part of the embedding. The LLM is allowed to see labels when it picks lines.

## Why it's parked here

Not this effort. Ticket 04 only has to make search return hydratable indexes. LLM gate/extract and in-app search IPC need their own wayfinder.

## Flow (not built)

```
query (+ optional speaker=Alice)
  → keep chunks whose live lines include Alice     // metadata, like today’s tag filter
  → cosine on the vector                           // words only, no speaker prefix
  → load history.jsonl by note_id
  → take segments[chunk.segment_indexes]
  → drop non-Alice lines                           // before the LLM, not after
  → LLM: is this chunk relevant? true / false      // sees speaker + timestamp + text
  → LLM: [0, 2]                                    // indexes in that (already filtered) window
  → return those lines as cited context
```

Homogeneous chunks (one speaker, then a size ceiling) make the filter mean “this vector is Alice’s words.” If a window were mixed, matching on Bob then throwing Bob away is the leak this order avoids.

Silence-triggered Whisper is a **capture** idea, not a chunk cut: [ticket 06](../../.scratch/context-chunking-strategy/issues/06-silence-triggered-whisper.md), also [streaming-dictate-transcription.md](streaming-dictate-transcription.md). Indexing still waits until Stop.

## Suggested future destination

1. Land tickets 03–05 so the index actually stores ranges (this effort).
2. New effort: local LLM gate + extract-indexes on CLI search hits; measure latency on-device.
3. Only then: in-app IPC / context pack that cites lines, not a flattened snippet.

## Source dump (2026-08, exploration)

Raw ask this note grew from:

Assuming speaker segments and timestamps (I know currently in the front end we group these) I would like to explore how we can develop a chunking strategy that groups speaker and timestamps and characters to set up for embedding and vectorisation.

… silence as a way to break up transcription processing so we don’t wait until the end …

… the vectors that are semantically retrieved need to point back to the text that includes the speaker and the timestamps line by line originally captured in Whisper … query first matched to the vector, then gated by an LLM, then another LLM extracts the applicable lines to reduce the noise. That’s all in the future. Right now we’re focusing on the chunking strategy …
