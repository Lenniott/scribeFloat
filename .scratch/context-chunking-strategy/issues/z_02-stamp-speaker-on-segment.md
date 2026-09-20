---
title: Stamp speaker on each segment
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-agent
blocked_by: ["01-adr-segment-is-the-transcript-atom.md"]
parent: MAP.md
---

Status: closed (2026-08-18)

# 02 — Stamp speaker on each segment

**What to build:** After a Record or Upload transcript is finalized, every Whisper line carries who spoke (identity slot or In/Out channel). Old notes missing the field still load. The existing turn list is still written (expand — do not delete it here).

**Blocked by:** ADR — segment is the transcript atom

- [x] Alignment / channel labeling writes `speaker` onto each segment at the same moment today’s turn list is built
- [x] Dictate and failed diarization leave speaker unset (or a single unlabeled value); the Note still saves
- [x] Notes from before this field deserialize; missing speaker is treated as unlabeled
- [x] A new Record note’s jsonl is inspectable: lines have speaker, and the old turn list is still present
- [x] Relabel still works as today (it may keep writing the turn list until ticket 03)
- [x] Clippy `-D warnings` and unit tests pass

## Resolution

`Segment.speaker: Option<String>` is stamped in `align_ranges_to_segments` (identity / `Other`) and `build_channel_blocks` (`In` / `Out`) at the same moment `speaker_blocks` are built. Dictate, no-evidence, and failed diarization leave it unset. Relabel still edits the turn list only. Expand complete — 03/04 can run.
