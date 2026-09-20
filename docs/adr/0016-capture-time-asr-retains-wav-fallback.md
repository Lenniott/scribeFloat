# Capture-time ASR retains a complete WAV fallback

**Status:** Binding
**Wayfinder:** Dictate latency investigation (`.scratch/dictate-latency/`), user-authorized implementation, September 2026.

Dictate and Record microphone capture may transcribe completed audio spans while capture continues,
using a shared capture-time ASR service. The writer-thread tap queues bounded,
WAV-equivalent PCM; a separate worker detects pauses and serializes inference
through the existing model service. Start with at least 30 seconds of context and
a confirmed pause before closing a span: shorter jobs changed words in fixture
comparisons. This reduces work remaining at Stop, with some additional total
computation. It does not promise identical wording across all possible boundaries.

Keep the complete capture WAV until transcription has succeeded. A missing VAD,
worker failure, queue overflow, or an unbroken span exceeding the memory limit
invalidates the entire live result and uses the existing full-WAV path. Cancel
discards queued work; workers never paste, save Notes, or change UI state. Stop
drains capture and joins the worker before final assembly. Short recordings remain
a single ASR job with the same model and decoding settings.

ASR jobs produce ordinary `Segment`s with absolute timestamps. They introduce no
new persisted transcript or retrieval-chunk schema; ADR-0015's freeze-after-Stop
rule holds. The service is shared infrastructure under ADR-0003. Dictate and
Record feed it from the mic writer tap. Record also feeds its existing Sortformer
worker; speaker stamping runs only after final diarization ranges and ASR are
available. Missing diarization still degrades to a plain transcript.

Record reuses completed mic segments in the shared post-capture pipeline. When
system audio has signal, its assembled WAV still transcribes after Stop, then the
existing chronological merge and In/Out channel labels apply. This preserves
loopback enable/disable gaps without adding a second streaming clock. Silent or
unavailable loopback retains mic diarization. Upload scheduling is unchanged.
Cancel, WAV-only save, and shutdown cancel mic ASR; the worker never persists a
partial Note. Model weights and inference remain local.
