---
title: Verify all bundled models before load
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Do Whisper, VAD, and Sortformer all get the same integrity check before use after they are copied into the writable models folder (hash and/or re-seed from the signed app bundle on mismatch)?

**Done when:** Sortformer is not “filename only”; bad/missing copies fail clearly offline (no network redownload).

## Spec (to-spec)

### Problem Statement

The app ships three models in the signed bundle and copies them into a writable models folder: Whisper Small (transcript), Silero VAD (voice activity), and Sortformer (speaker turns on Record / Upload). Whisper and VAD already refuse to run if the on-disk copy fails a SHA-256 check, and they tell the user to reinstall — offline, no download. Sortformer only checks “file exists and is non-empty,” then loads it into ONNX. A tampered or half-copied Sortformer file in the models folder can still run. Startup seeding also skips copy when the destination already exists, so a bad file is never overwritten from the signed bundle. That is uneven trust for the same class of artifact.

### Solution

Treat all three bundled models the same way before use: known SHA-256, checked before load, no network fetch if something is wrong. Sortformer joins Whisper and VAD. Bad or missing copies surface a clear offline / reinstall story (or, if we choose re-seed, try once from the signed bundle then fail clearly if still wrong). Speaker labeling may still soft-skip when the model is simply absent in a dev build with empty placeholders — that must not mean “load any non-empty bytes.”

### User Stories

1. As a Record user, I want speaker turns to come only from a trusted Sortformer file, so that a corrupted models folder cannot run arbitrary ONNX.
2. As an Upload user, I want the same Sortformer integrity rule as Record, so that bulk Notes are not a weaker path.
3. As a Dictate user, I want Whisper and VAD integrity unchanged, so that quick capture stays offline-safe and consistent.
4. As a Record user on a long clip, I want VAD to keep failing closed on hash mismatch, so that silence handling does not trust a bad file.
5. As anyone using transcription, I want Whisper to keep failing closed on hash mismatch, so that the transcript engine stays pinned to the bundled Small model.
6. As a user with a missing Sortformer file (dev placeholder / incomplete install), I want the app not to pretend a zero-byte or wrong file is fine, so that I get a clear offline message or intentional “no speakers” behaviour — never silent load of junk.
7. As a user whose models folder was altered, I want no Hugging Face or other runtime download to “fix” it, so that PRIVACY and bundle-only stay true.
8. As a user who reinstalls the app after a bad model, I want seeding from the signed bundle to restore good files, so that reinstall is the real recovery path.
9. As a Silicon tester, I want corrupt Sortformer to be catchable in a simple check, so that ship-bar smoke can trust the integrity story.
10. As an agent implementing this, I want one primary verify-before-load seam for Sortformer (mirroring ModelService), so that we do not scatter hash logic across controllers.
11. As a maintainer, I want the Sortformer hash constant to match the fetch script pin, so that build-time and runtime trust the same bytes.
12. As a Windows (later) user, I want the same hash constants and fail-closed behaviour, so that integrity is not macOS-only.
13. As a user mid-recording when diarization cannot load a trusted model, I want capture itself to keep working, so that losing speakers never kills the recording (today’s degrade path), unless the chosen aggression says otherwise for corrupt files.
14. As a reader of errors, I want wording that says reinstall / restore bundled models — not Settings → Models or “download again.”
15. As a security reviewer, I want S5 closed with evidence that Sortformer cannot load without a passing hash (or successful re-seed + hash).

### Implementation Decisions

- **Primary seam (preferred):** Sortformer integrity lives next to readiness/load on the diarization service — same idea as ModelService’s VAD/Whisper checks. Controllers keep calling start-live / full diarize; they do not grow their own hash code.
- **Whisper and VAD:** Already SHA-256 before use. This ticket does not redesign those paths; only confirm they remain the template and that Sortformer matches their offline failure language.
- **Sortformer constant:** Add a published SHA-256 for the bundled Sortformer filename, equal to the pin in the fetch-bundled-models script (`cc520901…`).
- **When to check:** Before any ONNX/Sortformer load (live session spawn and full-audio Upload pass). “Available” for diarization must mean present, non-empty, **and** hash OK — not merely non-empty.
- **Startup seed (depends on aggression):** Today seed copies only if the destination is missing. Aggression may extend seed to replace a hash-failing destination from the signed resource when the resource has real content (skip 0-byte placeholders). Never fetch from the network at runtime.
- **Shared hashing:** Prefer reusing the existing streaming file SHA-256 approach used for Whisper/VAD rather than inventing a second hasher. Exact module placement is an implementer detail; behaviour is “one hash story.”
- **Error / degrade policy (depends on aggression):** Missing Sortformer in dev may still mean “Record without speaker labels.” A **hash mismatch** must not load ONNX. Prefer clear log + same reinstall copy as VAD/Whisper when we refuse the file; do not widen into hard-failing the whole Record session unless aggression says so.
- **BGE / embeddings models:** Out of scope (knowledge layer out of this map).
- **No runtime download** to repair models (ticket 12 already removed VAD HF fetch).

### Aggression (agreed)

**(2) Re-seed then hash** — Offline self-heal from the *installed* app resources (not the DMG, not the network). If the writable copy is missing/empty/hash-wrong → copy from app resources when those files have real content → re-check hash. If still bad → clear reinstall message. No network.

### Testing Decisions

- Test **external behaviour** at the diarization readiness/load seam: wrong bytes → not considered available / load errors; correct hash constant accepted; zero-byte still unavailable.
- Do **not** require real ONNX inference in unit tests for this ticket (existing ignored hardware/model tests stay separate).
- Prior art: ModelService tests for VAD/Whisper integrity mismatch and reinstall-oriented error strings; diarization tests for missing / zero-byte / non-empty file.
- If aggression includes re-seed: prefer a small testable helper for “should replace dest?” over bootstrapping the full Tauri app in unit tests.
- `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` must pass.

### Out of Scope

- Runtime model download / Hugging Face
- Settings → Models UI
- BGE / retrieval / embeddings model verify
- Changing Sortformer tuning (CallHome vs DIHARD3)
- Per-window IPC (ticket 16)
- ADR aspirational wording (ticket 18)
- Intel-specific packaging

### Further Notes

- Evidence: security S5; fetch script already pins Sortformer; gap is runtime + writable models dir.
- Ticket 12 locked bundle-only; this ticket locks **integrity parity**.
- Map context: Apple Silicon confidence; offline/reinstall is the recovery story.

### Code cut (fixed once aggression agreed)

1. Add Sortformer SHA-256 constant aligned with fetch script.
2. Verify before Sortformer load; non-empty alone is insufficient.
3. Apply chosen aggression for seed / re-seed.
4. Regression tests for mismatch / zero-byte / missing.
5. Grep: no new runtime download URLs; Sortformer load path references the hash.

### Done when

1. Sortformer cannot load into ONNX without a passing integrity check (or successful re-seed + check under aggression 2/3).
2. Whisper/VAD integrity behaviour remains fail-closed offline.
3. Bad copies do not trigger network repair.
4. Tests + clippy clean; approach in Resolution.

## Resolution

Aggression **(2)** on `feature/0.3/embeds`.

| Cut | Result |
|---|---|
| Sortformer SHA-256 | `SORTFORMER_MODEL_SHA256` matches fetch script pin |
| Before ONNX load | `ensure_model()` — heal from resources, then hash; refuse load if still bad |
| Live Record | Integrity fail → warn + no speaker labels (capture continues) |
| Whisper / VAD | On integrity fail, offline restore from resources then re-check; else reinstall error |
| Startup seed | VAD + Sortformer: restore if missing/empty/hash-wrong; Whisper: fill if missing/empty (hash/heal on first use so launches don’t re-hash ~181 MB) |
| Helper | `services/bundled_models.rs` — shared hash / restore / ensure |
| Network | None |

**Verify:** `cargo test -p ScribeFloat` → 344 passed, 5 ignored; `cargo clippy -p ScribeFloat -- -D warnings` clean.

## Comments

- 2026-07-19: claimed; `/to-spec` published; human chose aggression **(2)** after clarifying pack-time vs runtime trust + self-heal from installed app; implemented and closed.
