---
title: "Triage: Dual audio vs how many speakers we can get"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Dual audio vs how many speakers we can get" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

**Confirmed: the note is accurate.** Today, dual-source (Record with speaker capture) never invokes Sortformer at all. Speaker labeling is purely channel-based: every mic segment → "Mic", every speaker segment → "Speaker".

Path traced:

- `src-tauri/src/services/transcription.rs:154` — `dual_source = input.audio.speaker_pcm_16k.is_some()`.
- `src-tauri/src/services/transcription.rs:241-301` (`transcribe_capture_with_inference`) — when `speaker_pcm_16k` is `Some`, it runs **two separate Whisper ASR passes** (mic PCM, then speaker PCM — no Sortformer involved at all) and merges the resulting segment lists chronologically via `inference.merge_dual_source(...)` (line 286).
- `src-tauri/src/services/model.rs:821-866` (`ModelService::merge_dual_source`) — merges mic + speaker segment lists by `start_ms`, drops near-duplicate text (echo/bleed between the two captures, ≤1500ms apart with matching trailing text), and tags each output `Segment` with `source: SegmentSource::Mic` or `SegmentSource::Speaker` (`src-tauri/src/types.rs`). This is a 2-way channel tag, not a 4-or-8-way speaker id.
- `src-tauri/src/services/transcription.rs:306-335` (`build_speaker_result`) — explicitly branches: `if dual_source { return build_channel_blocks(segments); }` (line 315-316), **skipping** the `align_ranges_to_segments` / Sortformer-range path entirely. That Sortformer path (`speaker_align.rs`, up to 4 speakers) only runs in the single-source (mic-only) case, either from live-captured ranges (`SpeakerEvidenceInput::LiveRanges`) or an on-demand diarize pass (`SpeakerEvidenceInput::DiarizeOnDemand`, `transcription.rs:175-203`).
- `src-tauri/src/services/speaker_blocks.rs:43` (`build_channel_blocks`) — converts segments to `SpeakerBlock`s labeled by `segment.source` (Mic/Speaker), i.e. exactly "channel" labeling, confirming the ticket's description.
- Sortformer diarization itself (`src-tauri/src/services/diarization.rs`) is written to process **one PCM stream** and emit up to 4 `speaker_id` slots (0-3) — nothing about it is intrinsically single-file; it could in principle be run once per source (mic, speaker) and the two label sets merged/offset (e.g. speaker_id 0-3 for mic, 4-7 for speaker-side) to approximate up to 8 total identities. No such merge exists today.

**What a change would touch** (if this becomes Now/Later work):
1. `transcription.rs::build_speaker_result` — replace the `dual_source` short-circuit with a path that diarizes both mic and speaker PCM (reusing `SpeakerEvidenceInput::DiarizeOnDemand` twice, or a new dual variant) and remaps one side's `speaker_id` range to avoid collision.
2. `speaker_align.rs::align_ranges_to_segments` — would need to accept/align two segment×range pairs (mic segments against mic ranges, speaker segments against speaker ranges) rather than one.
3. `speaker_blocks.rs` — merge logic combining channel identity AND per-channel speaker id into a final label (e.g. "Speaker 3" instead of generic "Speaker"/"Mic").
4. UX/product question (explicitly deferred by the original note): does a "Speaker 5" derived from the system-audio channel read as meaningful to users, or is channel labeling actually preferable? This is a product call, not something this investigation should resolve.
5. Test coverage: `transcription.rs` has existing dual-source unit tests (`dual_source_returns_channel_blocks_and_never_diarizes`, line ~730) that assert the CURRENT behavior and would need rewriting.

**Size estimate: Medium.** Not a one-line fix — touches 3-4 files, adds a second diarization pass (2x Sortformer inference cost when dual-source + diarize-on-demand), and has real test/architecture surface. The product-facing question (is per-channel-then-merged labeling even desirable) is undecided and blocks a "just do it" implementation; recommend treating as its own scoped follow-up rather than folding into this merge effort.
