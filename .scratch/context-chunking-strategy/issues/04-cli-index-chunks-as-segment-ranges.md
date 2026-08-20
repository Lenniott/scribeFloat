---
title: CLI index stores segment ranges not passage copies
labels: [wayfinder:task, ready-for-agent]
status: open
assignee: agent-04
blocked_by: ["02-stamp-speaker-on-segment.md"]
parent: MAP.md
---

Status: ready-for-agent

# 04 — CLI index: ranges, not copies

**What to build:** After a Note is frozen, `index build` packs consecutive same-speaker lines up to the size ceiling, embeds the words **without** speaker prefixes, and stores only `{id, note_id, segment_indexes}` plus the existing binary vector file. `index search` can optionally filter to a speaker (same idea as today’s tag filter), hydrates lines from the Note jsonl, and prints indexes plus a snippet. Written notes and unlabeled Dictate still index.

Does **not** wait on the UI ticket — chunking reads `speaker` on the line.

**Blocked by:** Stamp speaker on each segment

- [ ] Schema version bump; an old index fails with a rebuild message
- [ ] Chunk rows do not persist the passage or an embed-input string; vectors stay in the binary sidecar (not JSON)
- [ ] Same-speaker runs stay together; a long monologue splits only on the size ceiling — silence/ASR job boundaries are not chunk cuts
- [ ] Embedding input is the concatenated line text with no speaker names
- [ ] `--speaker Alice` (or equivalent) keeps only chunks that include that label, resolved from **live** line speakers so a relabel without rebuild still filters correctly
- [ ] Search/context-pack output cites `note_id`, segment indexes, and a snippet hydrated from the Note
- [ ] Written notes and Dictate (no speaker) still produce a working index
- [ ] Clippy `-D warnings` and unit tests pass
