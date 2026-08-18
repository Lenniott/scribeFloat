---
title: Transcript UI and relabel use segments
labels: [wayfinder:task, ready-for-agent]
status: open
assignee:
blocked_by: ["02-stamp-speaker-on-segment.md"]
parent: MAP.md
---

Status: ready-for-agent

# 03 — Transcript UI and relabel use segments

**What to build:** Opening a Note still looks like today’s speaker turns (name, time range, paragraph). Those turns are grouped from consecutive same-speaker lines, not from a second copy of the words. “This turn” and “all turns named X” write speaker on the underlying lines. Markdown / export uses the same grouping.

**Blocked by:** Stamp speaker on each segment

- [ ] Record/Upload notes with labeled lines render turns identical in structure to today’s panel (label, range, body)
- [ ] Notes with no speaker labels still render as unlabeled paragraphs from the lines
- [ ] “This turn” retags only the lines in that consecutive group; “all turns” retags every line with that label
- [ ] Preview and on-disk markdown (when enabled) match the grouped view
- [ ] Old notes that still have a stored turn list but no per-line speaker keep rendering (fallback until ticket 05)
- [ ] Clippy `-D warnings` and relevant UI/unit tests pass
