---
title: Stop persisting speaker_blocks
labels: [wayfinder:task, ready-for-agent]
status: open
assignee:
blocked_by:
  - "03-derive-speaker-turns-from-segments.md"
  - "04-cli-index-chunks-as-segment-ranges.md"
parent: MAP.md
---

Status: ready-for-agent

# 05 — Stop persisting speaker_blocks

**What to build:** New Notes no longer duplicate the transcript as a parallel turn list. UI, markdown, relabel, and the CLI index all read labeled lines. Old Notes that still have a stored turn list keep working.

**Blocked by:** Transcript UI and relabel use segments; CLI index stores segment ranges not passage copies

- [ ] New Record/Upload notes do not write a duplicate turn-list field (or write it empty)
- [ ] Relabel, transcript view, markdown/export, and index build all succeed on those new notes
- [ ] Old notes with a stored turn list and no per-line speaker still open and render
- [ ] No remaining write path treats the turn list as source of truth for words
- [ ] Clippy `-D warnings` and unit tests pass
