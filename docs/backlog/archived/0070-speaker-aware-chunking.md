---
id: "0070"
title: Speaker-aware mic chunking before Whisper
status: done
---

# Speaker-aware mic chunking before Whisper

Cut mic PCM at likely speaker handovers (pitch + loudness + silence union) before Whisper, so segments do not span two voices. Voiceprint / `speaker_blocks.rs` still identifies *who* spoke.

## Shipped

- `services/speaker_chunking.rs` — `find_cuts`, `split_pcm_owned`, snap-to-quiet, max-span guard
- `ModelService::transcribe_pcm_with_speaker_cuts` — multi-pass Whisper with offset merge
- Gated behind `Config.speaker_aware_chunking`; default off because the current Rust pitch implementation does not yet match the benchmark target
- `SessionManifest.speaker_cuts` observability
- Scribe mic path + file-upload transcribe mic path (not Dictate, not speaker loopback in v1)
- Bench: `cargo run --features bench --bin speaker-chunk-bench -- <wav>`; ignored unit test with `SPEAKER_CHUNKING_FIXTURE`

## Follow-ups

- Speaker loopback / dual-source chunking
- Required parity script vs Python `.f0.npz` at 16 kHz before enabling by default
