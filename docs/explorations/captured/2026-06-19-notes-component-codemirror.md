---
status: captured
date: 2026-06-19
produces: ADR-0006 through ADR-0009, docs/backlog/active/0044-0051
---

# Exploration: Notes component — CodeMirror + unified note editor

**Date:** 2026-06-19  
**Status:** Captured → decisions written to ADRs 0006–0009 and stories 0044–0051

---

## What this session resolved

### The core idea

A unified writing surface that replaces both the Scribe capture panel and the read-only `NoteDetailPane`. A Note is a bundle — typed text, transcript from recording, and Float metadata — and they should all be created, viewed, and edited in one place.

Entry point: `+ New Note` in the TitleBar → `/notes/new` → immediately creates a Note → redirects to `/notes/[id]`. Existing notes open directly at `/notes/[id]`.

---

## Layout

```
┌──────────────────────────────────────────────────────────────┐
│  ← Notes   [EditableTitle]                                   │
├──────────────────────────────────────────────────────────────┤
│  ~~~waveform~~~  ● 00:42  [Stop & Save]  [🗑]  ⚙            │
├──────────────────────────────┬───────────────────────────────┤
│  Written │ Transcript        │  Metadata                     │
│  ────────────────────        │  ─────────────────────        │
│  [CodeMirror editor]         │  Tags                         │
│                              │  Keywords                     │
│                              │  Float Layer Items            │
│                              │                               │
└──────────────────────────────┴───────────────────────────────┘
```

- **Recording chrome** (top strip, always visible): waveform, status dot, timer, Stop & Save, discard, gear icon → settings popover
- **Left panel**: Written editor tab OR Transcript viewer tab — switchable
- **Right panel**: Metadata sidebar — always present (the nudge surface)
- **Default pair**: editor + metadata when no transcript; transcript + metadata when no written content

### Settings popover (gear icon)
Mic selector · Model selector · Speaker capture toggle (live, mid-recording) · Timestamps toggle · Input/Output labels (stored in config, applied at render time — not yet wired to render path)

---

## Panel switching

- Show any two panels at once
- Left panel tab strip: Written / Transcript (Transcript tab hidden if no transcript)
- Default: Written tab if no transcript, Transcript tab if no written content
- Keyboard shortcuts for switching TBD in implementation

---

## Note lifecycle (ADR-0009)

| Situation | Behaviour |
|---|---|
| No title change + no content + no transcript + no metadata | Silent auto-delete on leave |
| Metadata set but nothing else | Prompt: "Discard or keep empty note?" |
| Any content present | Keep, already autosaved |
| Recording in progress | Keep note, recording continues in background |

Autosave: title at 500 ms debounce, written content at 800 ms debounce. No save button.

---

## Folder structure (ADR-0007)

Every Note gets a folder:
```
HHMM_DD-MM-YY_first_seven_words_XXXXXX/
  note.md          ← written source + transcript + YAML frontmatter (when auto-export on)
  mic.wav          ← kept after processing (no delete)
  speaker.wav      ← kept after processing
```

**ID generation:** 6-char base-36 from MD5 of `"{unix_timestamp} {7-word-title}"`.  
**Title cap:** 7 alphabetical words for folder naming only. Display title uncapped.

YAML frontmatter in `note.md`:
```yaml
---
title: 'My Note Title'
created: '2025-06-04T10:30:00'
tags: [tag1, tag2]
keywords: [kw1, kw2]
model: base
duration_seconds: 142.0
word_count: 847
---
```

---

## Editor (ADR-0008)

**CodeMirror 6** with `@codemirror/lang-markdown`. Source mode — user sees markdown syntax.

**Styling approach — CSS-first:**
- Heading sizes: CSS on `.cmt-heading1`–`.cmt-heading6` (free, no decoration work)
- Bold/italic: markers stay visible, text between them is styled
- Code: monospace + `bg-fill` background

**Explicitly deferred:** Obsidian-style marker-hiding on non-focused lines. Added per-element type later as standalone CM6 `ViewPlugin` files.

**Ruled out:**
- `@atomic-editor/editor` — Obsidian-style live preview for CM6. Too new, solo-maintained, maintenance risk.
- Milkdown Crepe — ProseMirror WYSIWYG (CM6 only for code blocks). Wrong paradigm.

**Transcript rendering:** `pulldown-cmark` on the Rust backend converts the transcript markdown → HTML. Served via new `note_render_transcript_html` IPC command.

---

## Markdown features — must-have vs deferred

| Feature | How | When |
|---|---|---|
| Heading font sizes | CSS on `.cmt-heading*` | Story 0045 (initial) |
| Bold / italic styling | CSS on `.cmt-strong`, `.cmt-emphasis` | Story 0045 (initial) |
| Code block box | CSS on `.cmt-monospace` + line decoration | Story 0045 (initial) |
| Checkbox widgets (`- [ ]`) | CM6 `WidgetDecoration` | Later story |
| Marker-hiding on leave | CM6 `ViewPlugin` per element | Later story |
| Tables (rendered) | CM6 widget | Later story |
| Mermaid diagrams | mermaid.js widget | Later story |

---

## What this replaces / closes

| Replaced | By |
|---|---|
| `CaptureView` overlay + `scribe.svelte` | Note editor recording strip (story 0046) |
| `NoteDetailPane` (read-only view) | Transcript panel in note editor (story 0048) |
| Story 0033 (Scribe screen redesign) | This work is the redesign |
| Flat `title_model.md` export | `note.md` inside Note folder (story 0050) |

---

## New IPC commands needed

| Command | Story |
|---|---|
| `note_create_empty() → NoteId` | 0049 |
| `note_delete(id)` | 0049 |
| `note_is_empty(id) → bool` | 0049 |
| `note_has_metadata(id) → bool` | 0049 |
| `note_save_written_source(id, content)` | 0045 |
| `note_attach_transcript(id)` | 0046 |
| `note_get_metadata(id)` | 0047 |
| `note_set_tags(id, tags)` | 0047 |
| `note_set_keywords(id, keywords)` | 0047 |
| `note_set_layer_items(id, items)` | 0047 |
| `note_render_transcript_html(id) → String` | 0048 |

---

## Build order

```
0044  Note editor shell (routes, layout, leave-guard stub)
  ├── 0045  CodeMirror written editor
  ├── 0046  Recording strip (reuses existing scribe_* commands)
  └── 0047  Metadata sidebar
        ├── 0048  Transcript panel + pulldown-cmark
        └── 0049  Note lifecycle (create, autosave, discard-if-empty)
                    └── 0050  Folder structure + markdown export
                              └── 0051  Notes list Written tab
```

Stories 0045, 0046, 0047 can be built in parallel once 0044 is done.

---

## Open questions not resolved this session

- Keyboard shortcuts for panel switching (left/right panel toggle) — TBD in implementation
- Speaker label substitution in rendered transcript — labels exist in Config but are not yet wired into the render path; deferred
- Whether the existing `CaptureView` overlay is removed immediately on launch of new editor or kept temporarily — suggested: keep behind a feature flag until 0046 is proven
