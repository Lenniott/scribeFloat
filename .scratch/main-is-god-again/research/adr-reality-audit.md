# ADR reality audit

Audit of `docs/adr/` against code on this branch (`feature/0.3/embeds` / current working tree). Classifications:

| Class | Meaning |
|-------|---------|
| **binding** | Decision is reflected in running code (evidence paths below) |
| **aspirational** | Decided in an ADR; not (or only stub-partially) built |
| **superseded** | Later ADR replaces it; keep for history |

**Recommend** is keep / mark / relocate only — no files deleted by this audit.

---

## Summary table

| ADR | Title | Class | Recommend |
|-----|-------|-------|-----------|
| README | Index | n/a | **keep** — still accurate as an index; add status column when ADRs are marked |
| 0001 | Note as primary domain object | **binding** | **keep** — note residual `HistoryRecord` / annotation-`Note` naming debt |
| 0002 | Note is a composition of Sources | **aspirational** | **mark** aspirational (partial: written + transcript coexist flatly) |
| 0003 | Scribe/Dictate are capture profiles | **binding** | **keep** — `CaptureProfile` + shared post-capture; dual controllers still deferred as ADR says |
| 0004 | Triage is per-Note | **aspirational** | **mark** aspirational; optional **relocate** pointer into Float explorations until Float ships |
| 0005 | Knowledge layer as markdown | **aspirational** | **mark** aspirational; **relocate** consideration — MAP lists knowledge layer out of scope |
| 0006 | Unified note editor | **binding** | **keep** — residual `NoteDetailPane` for legacy list items only |
| 0007 | Note folder structure + ID | **aspirational** | **mark** aspirational — code comments cite story 0050 interim |
| 0008 | CodeMirror for written editor | **binding** | **keep** |
| 0009 | Note lifecycle (create/autosave/discard) | **binding** | **keep** |
| 0010 | Capture config ≠ note intent | **aspirational** | **mark** aspirational — `CONTEXT.md` already describes target; code still `HistoryKind` |
| 0011 | Voiceprint engine | **superseded** | **keep** as history (already points at 0014); do not delete |
| 0012 | Navigation intent via appState flag | **binding** | **keep** |
| 0013 | Live pitch analysis + cut storage | **binding** | **keep** — optionally annotate stale “identity = voiceprint” consequence vs 0014 |
| 0014 | Anonymous diarization replaces voiceprint | **binding** | **keep** |

**Counts:** binding 8 · aspirational 5 · superseded 1 · README n/a

---

## Per-ADR evidence

### ADR-0001 — Note as primary domain object — **binding**

**Decision:** Note (mutable knowledge unit) replaces archival `HistoryRecord` as the central concept.

**Evidence (matches):**
- UI/routes treat notes as first-class: `src/routes/notes/[id]/+page.svelte`, `src/routes/notes/new/+page.svelte`, `src/lib/ui/5_views/note-editor.svelte`
- Mutable persistence paths: `note_create_empty`, `note_save_written_content`, `note_save_title`, `attach_transcript` in `src-tauri/src/commands/history.rs` + `src-tauri/src/services/history.rs`
- Glossary: `CONTEXT.md` § Core domain object

**Gaps (debt, not reclassification):**
- Canonical Rust type remains `HistoryRecord` (`src-tauri/src/types.rs` ~599–653: “canonical, source-of-truth record”)
- Conflicting name: small session-annotation struct also called `Note` (`types.rs` ~287–292)

**Recommend:** keep.

---

### ADR-0002 — Note is a composition of Sources — **aspirational**

**Decision:** Notes are composed of addressable Sources (`transcript`, `written`, `upload_audio`, `web`, …), not a flat blob.

**Evidence (partial only):**
- Flat fields on `HistoryRecord`: `written_content`, `segments`, `speaker_blocks` — not a `sources: Vec<Source>` model (`types.rs` ~602–653)
- Written + transcript can coexist via `TranscriptAttachment` / `attach_transcript` (`types.rs` ~655–673; `history.rs` attach path)
- Glossary lists Source types including future `web`/`video`/`import_md` (`CONTEXT.md`) — no implementing types in `src-tauri/src`

**Missing:** no Source type enum / individually addressable source IDs for Float per-source processing.

**Recommend:** mark aspirational (partial coexistence of written + transcript).

---

### ADR-0003 — Scribe and Dictate are capture profiles — **binding**

**Decision:** Same capture stack; differences are profile config. Dual controllers are historical debt; do not widen the gap. Unification deferred.

**Evidence:**
- Shared `CaptureProfile { Record, Upload, Dictate }` + `PostCaptureInput` / `run_post_capture_transcription` — `src-tauri/src/services/transcription.rs` ~32–73
- Controllers still split: `ScribeController` (`controllers/scribe.rs`), `DictateController` (`controllers/dictate.rs`) — matches ADR’s explicit “not done yet”
- Both call shared post-capture with `CaptureProfile::Record` / `::Dictate`

**Recommend:** keep. Unification remains future work *inside* this ADR’s consequences, not a separate aspirational ADR.

---

### ADR-0004 — Triage is per-Note — **aspirational**

**Decision:** Triage status lives on the Note; Float results merge into one triage surface.

**Evidence (absent):**
- `rg triage|TriageStatus` over `src/` + `src-tauri/src` → no product Triage model
- Float / triage described in docs/explorations and `CONTEXT.md` navigation copy; not implemented as data or UI workflow

**Recommend:** mark aspirational; optional relocate note into Float exploration docs until Float phase starts.

---

### ADR-0005 — Knowledge layer stored as markdown — **aspirational**

**Decision:** Domains/Artifacts as markdown + YAML frontmatter folders, not a DB.

**Evidence (absent in product code):**
- `CONTEXT.md` ~139: “The knowledge layer is a future phase… not yet built”
- MAP out of scope: `.scratch/main-is-god-again/MAP.md` Out of scope → knowledge layer
- No Artifact/Domain store under `src-tauri/src` or `src/`

**Recommend:** mark aspirational; consider relocating detail into `docs/explorations/` and leaving a one-line ADR stub that points there — do not delete the ADR.

---

### ADR-0006 — Unified note editor — **binding**

**Decision:** `/notes/[id]` replaces Scribe overlay + read-only detail as primary surface.

**Evidence:**
- Routes: `src/routes/notes/new/+page.svelte` → `note_create_empty` → `/notes/[id]`
- Editor: `src/lib/ui/5_views/note-editor.svelte` (written CM + transcript/metadata panels)
- `captureOpen` / `CaptureView` — no matches in `src/` (overlay retired for this flow)

**Residual:**
- `NoteDetailPane` still used for legacy list items (`src/lib/ui/5_views/notes.svelte` — non-editor notes) and design-system gallery

**Recommend:** keep.

---

### ADR-0007 — Note folder structure and ID generation — **aspirational**

**Decision:** Per-note folder `HHMM_DD-MM-YY_<title>_<base36id>`; durable audio + `note.md` inside.

**Evidence (not implemented; interim admitted in code):**
- `src-tauri/src/services/note_sidecar.rs` lines 1–5: “Interim until story 0050 per-note folders replace `.notes/` with ADR-0007 folder names”
- Actual layout: `{save_folder}/.notes/{id}/written.md` + `meta.json`
- Session audio still via `make_session_dir` / `session_dir` on records (`controllers/scribe.rs`, `HistoryRecord.session_dir`)
- No MD5 / base-36 folder ID generator found

**Recommend:** mark aspirational; keep ADR as the target for story 0050.

---

### ADR-0008 — CodeMirror for written source editor — **binding**

**Decision:** CodeMirror 6, source-mode markdown, CSS-first styling.

**Evidence:**
- Deps: `package.json` `@codemirror/commands`, `lang-markdown`, `state`, `view`
- Component: `src/lib/ui/2_components/controls/MarkdownEditor.svelte` (EditorView, markdown lang, theme)
- Wired in editor: `note-editor.svelte` imports `MarkdownEditor`

**Recommend:** keep.

---

### ADR-0009 — Note lifecycle — **binding**

**Decision:** Immediate create; ~800 ms autosave; discard-if-empty; recording exception; default timestamp title.

**Evidence:**
- Create: `note_create_empty` (`commands/history.rs` ~135–142); `notes/new/+page.svelte`
- Autosave 800 ms: `note-editor.svelte` ~168–173 → `note_save_written_content`
- Leave guard: `src/lib/services/noteLeaveGuard.ts` (`note_is_empty` / `note_has_metadata` / `history_delete`; recording proceeds)
- Default title format in history controller / sidecar: `%H:%M %d/%m/%y`

**Recommend:** keep.

---

### ADR-0010 — Separate capture config from note intent — **aspirational**

**Decision:** Replace `HistoryKind` with `quick: bool` + `origin: mic|upload`; UI/settings use quality tiers; rename Scribe→Record in model layer.

**Evidence (not done):**
- `HistoryKind { Scribe, Dictate, Transcribe, Written }` still on `HistoryRecord` (`types.rs` ~592–605)
- UI still labels by kind: `src/lib/services/historyFormat.ts` `kindLabel`; `NoteCard.svelte` / `RecentNoteCard.svelte`
- `SourceIcon.svelte` still branches on `dictate` / `transcribe` kinds, not `origin`
- Controllers/commands still `scribe_*` / `ScribeController` (not `RecordController`)
- No “Fast model” / “Refined model” settings labels found in `src/`
- Glossary already describes target (`CONTEXT.md` Note Metadata `quick` / `origin`) — docs ahead of code

**Recommend:** mark aspirational; keep as binding *intent* for a future rename ticket.

---

### ADR-0011 — Voiceprint engine — **superseded**

**Decision (historical):** sherpa-onnx campplus binary verification; threshold 0.75; biometric profiles.

**Evidence:**
- ADR front matter already: “Superseded by ADR-0014”
- No `sherpa-onnx` / campplus in `src-tauri/Cargo.toml` or active services
- Purge path only: `src-tauri/src/services/legacy_voice_purge.rs`; `platform/mod.rs` deletes legacy keychain key
- Config comment: retired voiceprint keys ignored (`types.rs` ~117–118)

**Recommend:** keep file as history; do not delete.

---

### ADR-0012 — Navigation intent via shared state flag — **binding**

**Decision:** Short-lived `appState.scribeAutoStart` boolean across `goto` → `onMount`.

**Evidence:**
- Flag: `src/lib/stores/appState.svelte.ts` `scribeAutoStart`
- Set before navigate: `src/lib/ui/6_regions/TitleBar.svelte` ~57
- Consume/clear: `src/lib/ui/5_views/note-editor.svelte` ~127–128

**Recommend:** keep.

---

### ADR-0013 — Live pitch analysis and change-cut storage — **binding**

**Decision:** `pitch-detection` (McLeod) on writer-thread `Pcm16kTap`; cuts on record; timeline in `analysis.json`.

**Evidence:**
- Crate: `src-tauri/Cargo.toml` `pitch-detection = "0.3"`
- Pure module: `src-tauri/src/services/analysis.rs` (`PitchAnalyzer`, `detect_cuts`)
- Tap type: `src-tauri/src/services/audio.rs` `Pcm16kTap`
- Persist timeline: `services/output/session.rs` `analysis.json`; harvest in `controllers/scribe.rs`
- Durable cuts: `HistoryRecord.speaker_change_cuts` (`types.rs` ~614–618)

**Stale text (not reclassification):** consequences still say identity is voiceprint (ADR-0011); ADR-0014 replaced that. Cuts remain identity-free enrichment.

**Recommend:** keep; optional one-line consequence edit when someone next touches the ADR.

---

### ADR-0014 — Anonymous diarization replaces voiceprint — **binding**

**Decision:** Sortformer live diarization; plain `speaker_names`; purge biometrics; supersedes 0011.

**Evidence:**
- Model crate: `Cargo.toml` `parakeet-rs` with `sortformer` feature
- Service: `src-tauri/src/services/diarization.rs`; live session from scribe mic tap (`controllers/scribe.rs`)
- Alignment: `src-tauri/src/services/speaker_align.rs` (max overlap → label; else Other)
- Names: `services/speaker_names.rs`, `commands/speaker_names.rs`, `note_relabel_speaker` (`commands/history.rs`)
- Dictate skips diarization (comment in `controllers/dictate.rs`)
- Purge: `legacy_voice_purge.rs` + startup in `lib.rs`
- UI: `TranscriptPanel.svelte` relabel + `speaker_names_list`

**Recommend:** keep.

---

## Cross-cutting notes for maintainers

1. **Glossary vs code:** `CONTEXT.md` already describes ADR-0010’s `quick`/`origin` and ADR-0002 Sources — treat glossary as target model where ADRs are aspirational.
2. **Do not delete superseded ADRs** (0011); they explain purge/legacy deserialize tests.
3. **Highest-value marks:** 0002, 0007, 0010 (agents will otherwise assume folder naming / Source types / HistoryKind removal already shipped).
4. **No product code changed** by this audit.
