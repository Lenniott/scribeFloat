# ADR-0010: Separate Capture Configuration from Note Intent

**Status:** Aspirational
**Wayfinder:** pre-wayfinder / orphan — `quick`/`origin` vs `HistoryKind` migration not shipped; revisit in a future wayfinder (deferred as Known issue "Record vs Dictate naming / dual-controller honesty" during the closed "Main is God again" effort — see `docs/ideas/main-is-god-again-known-issues.md`).

## Context

ScribeFloat has two capture activation surfaces — Record (in-app long-form recording; formerly "Scribe") and Dictate (hotkey-triggered quick capture) — which have historically been treated as distinct Note *types* as well as distinct capture *modes*. This conflation created several problems:

- "Scribe" and "Dictate" were doing two jobs at once: configuring how intake happens *and* implying the intent or type of the Note produced.
- `SourceIcon`, `Chip`, and `NoteCard` encoded capture mode (`dictate`, `transcribe`) as Note metadata, coupling the UI mental model to an implementation detail.
- Float flows had no clean way to target Notes by intent — they would have had to target by capture mode instead, which is the wrong level of abstraction.
- Upload (external sources) was a third capture surface with no clear relationship to the other two in the domain model.

The root confusion: capture configuration (how intake happens) was tangled with note intent (what the content is and how Float should treat it).

## Decision

We will separate capture configuration from Note intent across the domain model and UI.

**Capture is intake configuration only.** The three capture surfaces are **Record** (long-form in-app recording, formerly "Scribe"), **Dictate** (hotkey quick capture), and **Upload** (external source import). These are verbs — actions the user takes — not Note types or domain objects.

**One bundled Whisper Small model** handles transcription for Record, Dictate, and Upload. There is no in-app model chooser and no Settings → Models surface. If the bundled model is missing or corrupt, the user reinstalls the app — the product does not offer quality tiers, per-path model assignment, or runtime model download.

**`HistoryKind` is replaced by two fields on `HistoryRecord`:** *(target — **not fully implemented**; code still uses `HistoryKind`)*

- `quick: bool` — true if captured via the fast/solo path (formerly Dictate). Drives the "quick" Chip on NoteCard and gates markdown export (quick Notes are plain text, not re-renderable transcripts).
- `origin: "mic" | "upload"` — where the audio came from. Drives `SourceIcon`. `mic` for all live recordings (Record and Dictate); `upload` for imported files.

Old Notes on disk lacking these fields default to `quick: false`, `origin: "mic"` — correct for legacy Scribe records.

**`SourceIcon` encodes origin, not capture mode.** The distinction worth showing is `mic` (live recording) vs `upload` (imported file). Capture mode is not surfaced here.

**Float flows are assigned via filters on Notes.** Flows target Notes through filter criteria (tags, origin, `quick` flag, Layer assignments) — not by capture type. Intent classification is a Float concern, not a capture-time declaration.

**Three source contexts exist but are not named capture types.** Users implicitly bring three kinds of content into ScribeFloat: solo quick capture (replacing typing), live long-form recording (conversations, self-narration), and external sources (uploaded files, URLs). These contexts inform Float flow design but do not require a `kind` field on the Note. Float infers or filters for them.

## Consequences

**Easier (when fully implemented):**
- Note mental model is clean: a Note is a Note regardless of how it was captured.
- Float flow assignment is more flexible — filters on any Note property, not locked to capture mode.
- `SourceIcon` and `Chip` vocabulary is stable — `mic`, `upload`, `quick` don't need to change as capture surfaces evolve.
- Adding a new capture surface (e.g. browser extension, API ingest) doesn't require a new Note type.

**Harder (pending migration):**
- `HistoryKind` enum (`Scribe`, `Dictate`, `Transcribe`) is removed from `types.rs`; replaced by `quick: bool` and `origin: NoteOrigin` (`mic | upload`) on `HistoryRecord`. Old serialized records lacking these fields default to `quick: false`, `origin: "mic"`.
- `kindLabel()` in `historyFormat.ts` is removed; NoteCard meta line no longer shows a capture mode label.
- `SourceIcon` prop renamed from `kind` to `origin`; accepts `"mic" | "upload"`.
- `ScribeController` → `RecordController`; `commands/scribe.rs` → `commands/record.rs`; `commands/transcribe.rs` → `commands/upload.rs`.
- Onboarding copy that presents "Scribe" and "Dictate" as feature names needs revision.

**Implementation status (current spine):** Capture verbs (Record/Dictate/Upload), shared post-capture transcription, and the bundled Whisper Small model are in place. The `HistoryKind` → `quick`/`origin` schema migration, UI decoupling, and controller/command renames above are **not fully built**.

**Explicitly out of scope:**
- Renaming Dictate or Upload as activation surfaces — they keep their verb names. Only "Scribe" is retired, replaced by "Record".
- In-app model selection or Settings → Models — superseded by the one bundled Whisper Small product decision (tickets 08/12).
- Defining Float flow filter syntax — that is a Float-phase decision.
