# ADR-0011: Tags replace Tags-and-Keywords; tags carry per-note annotation logs

## Status

Accepted

## Context

The original Float enrichment design (design-brain-prd.md) proposed two default seed Layers: **Tags** and **Keywords**. Tags were broad categorical labels; Keywords were finer-grained domain terms. Both were flat string lists stored on the HistoryRecord.

During the knowledge-orchestration exploration it became clear that this distinction is artificial. In practice both concepts serve the same purpose — linking a note to a topic — and splitting them doubles the extraction work, complicates the UI, and muddies the vocabulary. The only meaningful difference was granularity, and granularity is better handled by how a tag is named than by maintaining two parallel vocabularies.

At the same time, the exploration clarified what Float should output per tag beyond just the tag name: a short annotation recording *why* this tag was applied to this specific note, anchored to the exact passage in the transcript. This annotation (stored as a log entry on the tag) is what makes context-file export useful — it lets the user take a structured, grounded summary to an external AI tool rather than a raw transcript dump.

## Decision

**Keywords are removed. There is only Tags.**

A Tag has:
- `name` — the term itself, shared across all notes
- `description` — what this tag means globally (optional, user- or Float-authored)
- `logs[]` — per-note log entries written by Float at processing time

Each log entry records:
- `note_id` — which note this log belongs to
- `timestamp` — Whisper segment timestamp in the source transcript (primary jump point into a long recording)
- `grep` — 2–4 distinctive words from the relevant passage (locates the exact sentence without asking the model to reproduce verbatim text)
- `status` — `starred` | `recent` | `archived` (starred = user flagged high signal; archived = deprioritised but kept; recent = automatic default)

Float's job when processing a note: extract relevant tags AND write a log entry for each — what in this note relates to the tag, anchored by timestamp and grep pattern. The log entry is the durable annotation; the grep+timestamp pair is how future agents and context-file exports navigate back to the source material.

## Consequences

- `HistoryRecord` gains `tags: Vec<Tag>` (replacing the former `tags: Vec<String>` and `keywords: Vec<String>`). The `Tag` type carries the log structure above.
- The `note_set_keywords` IPC command is not built. The `set_keywords` controller/service methods are not built.
- Story 0047 (metadata sidebar) is amended: the Keywords UI section and backend methods are dropped; only Tags remains.
- Story 0050 (note folder + markdown export): the `keywords:` YAML frontmatter field is dropped from `note.md`.
- ADR-0006 and ADR-0007 are updated to remove keywords references.
- The Float on-creation flow runs one step (Tags), not two (Tags + Keywords).
- The context-file export feature (deferred, knowledge-orchestration.md) becomes simpler: one log per tag, ordered by starred status, pulling the grep/timestamp anchor for each note.
- Existing notes without `tags` deserialise cleanly via `#[serde(default)]` — no migration needed.
