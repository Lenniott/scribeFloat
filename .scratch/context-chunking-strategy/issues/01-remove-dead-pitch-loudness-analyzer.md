---
title: Remove dead pitch/loudness change-cut analyzer (ADR-0013)
labels: [wayfinder:task]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

Status: ready-for-agent

## Question

`services/analysis.rs`'s `PitchAnalyzer` / `detect_cuts` / `CutConfig` /
`SpeakerChangeCut` (ADR-0013) was built to detect voice-change cuts as a
pre-diarization speaker-change signal. ADR-0014 (Sortformer diarization)
replaced the job it was doing. Should the now-orphaned parts be removed, and
ADR-0013 re-stamped?

## Evidence (2026-08-13 exploration session)

Traced every reference to `speaker_change_cuts` / `SpeakerChangeCut` /
`PitchAnalyzer` / `detect_cuts` / `CutConfig` across `src-tauri/src` and the
whole frontend (`src/`):

- **Computed live** on every Record session (`controllers/scribe.rs:892`) and
  every Upload/Transcribe pass (`controllers/transcribe.rs:475`), always via
  `CutConfig::default()` — `include_silence` is never set `true` anywhere, so
  the "useful for aggressive transcript chunking" capability the doc comment
  describes has never been reachable from any call site.
- **Stored durably**: `SessionManifest.speaker_change_cuts` (crash-recovery
  window) and `HistoryRecord.speaker_change_cuts` (every note, forever, in
  `history.jsonl`).
- **Sent over IPC**: `history_get_detail` returns the full `HistoryRecord`,
  cuts included.
- **Consumed by**: nothing found.
  - Zero hits for `speaker_change_cuts` / `SpeakerChangeCut` /
    `speakerChangeCuts` anywhere under `src/` (frontend). `TranscriptPanel.svelte`'s
    `NoteDetail` type doesn't even declare the field.
  - Not read by `chunk_records`/`context_search.rs` (embedding pipeline).
  - Not read by `services/output/render.rs` (markdown/transcript rendering).
  - Not read by diarization (`services/diarization.rs`,
    `services/speaker_align.rs`) — that path is fully independent, Sortformer-based.

**Not dead** — must survive any removal: `services/analysis.rs::rms()` is used
by `services/output/hallucination.rs` for hallucination-phrase gating. Only
the pitch/loudness/silence *cut-detection* machinery and the `SpeakerChangeCut`
plumbing are candidates, not the whole file.

## Scope if actioned

- Remove `PitchAnalyzer`, `detect_cuts`, `CutConfig`, `SpeakerChangeCut`,
  `CutReason`, and their call sites in `controllers/scribe.rs` and
  `controllers/transcribe.rs` (including `analysis.json` writing/reading and
  the `harvest_audio_analysis` helper).
- Remove `speaker_change_cuts` from `SessionManifest`, `HistoryRecord`,
  `TranscriptAttachment`, `PostCaptureInput`, `TranscriptResult` outright —
  no migration/compaction needed. Single user (Ben), and none of these
  structs use `#[serde(deny_unknown_fields)]`, so old `history.jsonl` lines
  still carrying the key just get silently ignored on parse once the field
  is gone from the struct. Update `attach_transcript`'s offset-shifting logic
  and the legacy-record test in `types.rs` accordingly — no shim field to
  preserve.
- Keep `services/analysis.rs::rms()` (relocate or keep the file, hallucination
  gating depends on it).
- Re-stamp `docs/adr/0013-live-pitch-analysis-and-change-cut-storage.md` to
  `Superseded` (ADRs are never deleted per `docs/agents/working-method.md`) —
  point it at whatever closes this ticket, and update the ADR index in
  `docs/adr/README.md`.
- `cargo clippy -- -D warnings` and `cargo test -p ScribeFloat` must pass.

## Comments
