# Handoff — context-chunking-strategy

**State right now:** Ticket 02 closed. Alignment and channel labeling stamp
`Segment.speaker` (`Speaker N` / `Other`, or `In` / `Out`). `speaker_blocks` are
still written. Dictate, no-evidence, and failed diarization leave speaker unset.
**Frontier is tickets 03 and 04, both in flight in parallel** (isolated worktrees).
Ticket 06 remains parked.

**What's next:** Wait for 03 (UI/relabel from consecutive same-speaker lines;
fallback to stored turn list for old notes) and 04 (CLI index as `note_id` +
segment indexes, not a passage copy). Orchestrator merges both, then 05 can
start. Do not stop persisting `speaker_blocks` until 05.

**Don't re-discover / re-litigate:**

- Read ADR-0015; do not revive stored `embed_text` / `lines`.
- Speaker in the embedding vs speaker as a filter — filter wins.
- How Whisper is scheduled does not matter if chunking waits until Stop + stamp.
- ASR job ≠ chunk. Do not change chunking in 03/04 except the index row shape in 04.
- Duplicate turn list is expand (02 done) → UI (03) → contract (05). Do not delete
  `speaker_blocks` until 05.
- Relabel must start writing `segment.speaker` in 03; until then it still edits
  the turn list only.
