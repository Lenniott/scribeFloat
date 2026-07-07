---
status: active
date: 2026-07-05
produces: future stories for types.rs invariants, transcript formatter, voice-embedding seam, frontend view state
---

# Architecture deepening candidates (deferred)

From the 2026-07-05 architecture review. Implemented (see git history for details):

1. **Concentrate the Note timeline shift** — **DONE 2026-07-05**: `TranscriptAttachment` + `HistoryRecord::attach_transcript` own the six-collection offset invariant.
2. **One capture module, three profiles** — **DONE 2026-07-05/06**: `services/transcription.rs` (`transcribe_capture`, `analyze_capture_speakers`) is now the shared pipeline for Record, Upload, **and Dictate**.

## Follow-up review: model loading & processing feedback (2026-07-05)

A second, scoped review ("model loading takes time, looks horrible") produced four candidates; three are **DONE 2026-07-06**:

- **Load models inside the recording window** — root cause was `reset_gpu_preference` evicting the cached Whisper context at every transcription start, making any warm-up pointless; now it only evicts genuinely CPU-fallback contexts. `ModelService::preload_context` + `VoiceprintService::preload_extractor` run in the recording window (Record: own model + ONNX; Dictate: its own smaller model only; Upload: ONNX alongside the first Whisper pass).
- **One processing-feedback interface** — one `ProcessingStage` vocabulary backend-wide (`DictateProcessingStage` deleted, wire-compatible), and one frontend mapper `src/lib/utils/processingFeedback.ts` (percent normalisation strips the 5% model-load headroom, indeterminate rules, step sequences per capture profile). TitleBar / dictate / transcribe views render only its output. Fixed a latent Upload bug: current stage displayed as already complete.
- **One model-download interface** — the voiceprint ONNX download now emits `ModelDownloadEvent` with `model_id = "voiceprint"` on the shared `model://download-progress` channel; `voiceprint://model-downloading` and `VoiceprintModelDownloadEvent` are deleted.

Still deferred from that review:

- **Per-item stage in the Upload queue** (speculative) — move `processing_stage` from `TranscribeStateEvent` onto `TranscribeQueueItem`; cheap now that the vocabulary is unified.

The four below are worth doing but deferred. Vocabulary: a module is *deep* when a lot of behaviour sits behind a small interface; a *seam* is where behaviour can vary without editing callers; deepening buys *locality* (bugs/tests concentrate in one place) and *leverage* (one implementation pays back across N call sites).

---

## 3. Encode the Note record's invariants (`types.rs`)

**Worth exploring.** `types.rs` is ~1,475 LOC of fully-public structs. Invariants are assumed everywhere and enforced nowhere:

- `segments` are assumed time-ordered (history.rs derives `offset_ms` from the last segment).
- `duration_ms` is derived from the last segment's `end_ms` but is also a freely-mutable field — nothing keeps them in sync.
- `SpeakerBlock.start_ms/end_ms/chunk_id` are `Option` with undocumented semantics (missing vs deliberately-unset vs legacy record).
- `SessionSpeaker.user_confirmed` has no interface governing when it may flip.
- The 17 existing tests in types.rs only cover serde round-trips and defaults, never invariants.

**Deepening:** constructors / mutation operations on `HistoryRecord` that validate; raw field mutation stops being possible from outside the module. The timeline-shift work (candidate 1) creates the first such operation — this candidate is its natural continuation.

**First step when picked up:** inventory every direct field mutation of `HistoryRecord` outside `types.rs` and classify which invariant each one silently relies on.

---

## 4. A Transcript formatter with one interface (`services/output/`)

**Worth exploring.** The output module is 2,599 LOC across 9 files with an interface nearly as wide as the implementation:

- Six per-capture-path format functions (`format_dictate_text`, `format_scribe_text`, `format_transcribe_text` × raw/replaced) instead of one operation parameterised by capture profile.
- Replacement rules travel as `&[ReplacementRule]` + prefix string through every caller instead of being held by a formatter constructed once from config.
- The seam leaks: `controllers/scribe.rs` imports `speaker_pcm_has_signal` and `SPEAKER_SILENCE_THRESHOLD` directly from `output/hallucination`.
- 26 tests cover individual submodules; nothing tests the full replace → de-hallucinate → dedup → render pipeline.

**Deepening:** a `TranscriptFormatter` built once from config + rules, exposing one `format(profile, segments)` operation. The eight submodules become internal seams tested behind it.

**Sequencing note:** do this *after* the capture-pipeline extraction (candidate 2) settles who calls the formatter — the pipeline should be its only caller.

---

## 5. A real seam for voice-embedding encryption — **DONE 2026-07-05**

Implemented as `services/voice_embeddings.rs`: one `VoiceEmbeddingStore` chosen at startup (`from_keychain()` — encrypted when the OS keychain key is available, plaintext otherwise), injected into both `HistoryService` and `VoiceprintService`, which lost their duplicated `Option<Arc<VoiceCryptoService>>` plumbing. Context strings moved byte-identical so existing encrypted data keeps decrypting.

Correction to the original framing: the encryption decision was never the `voice_embeddings_encryption_required` config flag — that flag only gates voice *learning* (settings). The adapter choice is keychain availability at startup.

---

## 6. Semantic view state over raw IPC mirroring (frontend stores)

**Speculative — deliberately parked.** Frontend stores (`src/lib/stores/`, 654 LOC) mirror backend IPC events field-for-field, so the backend event shape is effectively the interface for ~9,000 LOC of UI. Stores have zero tests. Illegal state combinations (e.g. recording *and* transcribing) are representable.

**Deepening:** derive semantic properties inside the store module (`isRecording`, `canPause`, formatted `recordingTime`) and have UI read only those; the IPC event shape becomes an implementation detail.

**Why parked:** candidate 2 will change the backend event shapes. Building a view-state seam on top of event shapes that are about to move would mean doing the work twice. Revisit once the capture pipeline is unified.

---

## Rejected during review (do not re-suggest without new evidence)

- **Auto-generating the IPC command layer** (`src-tauri/src/commands/`, 1,222 LOC of pass-throughs). Fails the deletion test payoff: Tauri owns the marshaling, the pass-throughs carry no invariants, and a macro layer would add its own interface to learn for near-zero leverage.
- **Merging the three speaker-analysis modules** (`analysis.rs`, `speaker_chunks.rs`, `speaker_blocks.rs`) into one pipeline. ADR-0013 deliberately separates pitch-cut detection from speaker identity ("a cut says the voice changed here — spans between cuts must not be presented as speaker identities"). The observed friction (scattered tuning constants) is real but too mild to reopen that decision. If the constants keep multiplying, the narrower fix is a shared `SpeakerAnalysisConfig`, not a merged module.
