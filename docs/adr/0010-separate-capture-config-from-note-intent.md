# ADR-0010: Separate Capture Configuration from Note Intent

## Status

Accepted

## Context

ScribeFloat has two capture activation surfaces — Scribe (in-app long-form recording) and Dictate (hotkey-triggered quick capture) — which have historically been treated as distinct Note *types* as well as distinct capture *modes*. This conflation created several problems:

- "Scribe" and "Dictate" were doing two jobs at once: configuring transcript quality *and* implying the intent or type of the Note produced.
- `SourceIcon`, `Chip`, and `NoteCard` encoded capture mode (`dictate`, `transcribe`) as Note metadata, coupling the UI mental model to an implementation detail.
- Model settings used "scribe" and "dictate" as the labels for model assignment roles, making it impossible to decouple quality tier from activation surface.
- Float flows had no clean way to target Notes by intent — they would have had to target by capture mode instead, which is the wrong level of abstraction.
- Upload (external sources) was a third capture surface with no clear relationship to the other two in the domain model.

The root confusion: capture configuration (how intake happens) was tangled with note intent (what the content is and how Float should treat it).

## Decision

We will separate capture configuration from Note intent across the domain model, UI, and settings.

**Capture is intake configuration only.** The meaningful dimension at capture time is transcript quality tier: *fast* (less accurate, lower latency) or *refined* (more accurate, higher latency). The user can set a default per tier in settings and override it at the moment of recording or upload. The three capture surfaces are **Record** (long-form in-app recording, formerly "Scribe"), **Dictate** (hotkey quick capture), and **Upload** (external source import). These are verbs — actions the user takes — not Note types or domain objects.

**`HistoryKind` is replaced by two fields on `HistoryRecord`:**

- `quick: bool` — true if captured via the fast/solo path (formerly Dictate). Drives the "quick" Chip on NoteCard and gates markdown export (quick Notes are plain text, not re-renderable transcripts).
- `origin: "mic" | "upload"` — where the audio came from. Drives `SourceIcon`. `mic` for all live recordings (Record and Dictate); `upload` for imported files.

Old Notes on disk lacking these fields default to `quick: false`, `origin: "mic"` — correct for legacy Scribe records.

**`SourceIcon` encodes origin, not capture mode.** The distinction worth showing is `mic` (live recording) vs `upload` (imported file). Capture mode is not surfaced here.

**Float flows are assigned via filters on Notes.** Flows target Notes through filter criteria (tags, origin, `quick` flag, Layer assignments) — not by capture type. Intent classification is a Float concern, not a capture-time declaration.

**Three source contexts exist but are not named capture types.** Users implicitly bring three kinds of content into ScribeFloat: solo quick capture (replacing typing), live long-form recording (conversations, self-narration), and external sources (uploaded files, URLs). These contexts inform Float flow design but do not require a `kind` field on the Note. Float infers or filters for them.

**Model settings expose quality tiers, not capture mode names.** Settings labels become "Fast model" and "Refined model". The "Dictate model override" concept is replaced by: a default model per quality tier, overridable per session.

## Consequences

**Easier:**
- Note mental model is clean: a Note is a Note regardless of how it was captured.
- Float flow assignment is more flexible — filters on any Note property, not locked to capture mode.
- Model settings are decoupled from activation surface names.
- `SourceIcon` and `Chip` vocabulary is stable — `mic`, `upload`, `quick` don't need to change as capture surfaces evolve.
- Adding a new capture surface (e.g. browser extension, API ingest) doesn't require a new Note type.

**Harder:**
- `HistoryKind` enum (`Scribe`, `Dictate`, `Transcribe`) is removed from `types.rs`; replaced by `quick: bool` and `origin: NoteOrigin` (`mic | upload`) on `HistoryRecord`. Old serialized records lacking these fields default to `quick: false`, `origin: "mic"`.
- `kindLabel()` in `historyFormat.ts` is removed; NoteCard meta line no longer shows a capture mode label.
- `SourceIcon` prop renamed from `kind` to `origin`; accepts `"mic" | "upload"`.
- `setting_models.svelte` labels change from "Default Scribe model" / "Dictate model override" to "Refined model" / "Fast model".
- `ScribeController` → `RecordController`; `commands/scribe.rs` → `commands/record.rs`; `commands/transcribe.rs` → `commands/upload.rs`.
- Onboarding copy that presents "Scribe" and "Dictate" as feature names needs revision.

**Explicitly out of scope:**
- Renaming Dictate or Upload as activation surfaces — they keep their verb names. Only "Scribe" is retired, replaced by "Record".
- Removing the fast/refined model distinction — this is preserved, just reframed.
- Defining Float flow filter syntax — that is a Float-phase decision.
