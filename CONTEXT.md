# ScribeFloat — Domain Glossary

> Canonical terms for this codebase. When code and this glossary disagree, fix one of them.
> Implementation details belong in `AGENTS.md` or `docs/architecture.md` — not here.

---

## Product

| Term | Definition |
|---|---|
| **ScribeFloat** | The product. The desktop app. |
| **Float** | The AI processing sub-system inside ScribeFloat — all enrichment workflows. Not a separate product. |
| ~~liscribe~~ | Retired name. Do not use. |

---

## Core domain object

| Term | Definition |
|---|---|
| **Note** | The primary information entity in ScribeFloat. A composition of one or more Sources, plus Metadata. What Float processes. What accumulates into the knowledge layer. |
| **Source** | An individual content piece inside a Note. Each Source has a type and origin. Sources are individually addressable — Float can process them separately or together. |
| **Note Metadata** | Structured fields on a Note: title, tags, Layer Item assignments, and any other Float-derived or user-assigned data. Editable by both user and Float. Includes `quick: bool` (captured via fast/solo path) and `origin: mic \| upload` (where audio came from). |
| **Transcript** | The text output produced by Whisper from audio capture. One type of Source. |

**Source types:**

| Source type | Origin |
|---|---|
| `transcript` | Audio capture via Scribe or Dictate — Whisper output |
| `upload_audio` | Imported audio file — Whisper output |
| `written` | User-typed markdown text |
| `web` | Scraped text from a URL (future) |
| `video` | Transcript extracted from a video URL (future) |
| `import_md` | Imported markdown file (future) |

All capture methods produce a Note. The Note is the unit of processing, storage, and recall.

**Audio input naming:** The distinction between mic and speaker audio is an implementation detail of the capture layer, not a domain term. Use `audio_input_type: mic | speaker` in code — not "source" — to avoid collision with the Note Source concept.

---

## Capture methods

Capture methods are ways of creating a Note. They are not distinct object types — they all produce the same thing.

| Term | Definition |
|---|---|
| **Record** | Long-form in-app recording. Durable audio (saved to Note folder). Refined transcription model by default. Stop confirmation safeguard. Previously called "Scribe." |
| **Dictate** | Quick-capture. Hotkey-triggered from anywhere. Temp audio (deleted on success). Fast transcription model by default. No stop confirmation. Output pastes to active app or clipboard. Produces a `quick` Note. |
| **Upload** | Bulk Note creation from external sources: audio files, markdown files, URLs, video URLs. Previously called "Transcribe." |
| **Recording** | The act of capturing audio. A UI state and verb-noun, not a domain object. Bounded by Start / Stop. |
| **Session** | One complete capture event — from initiation to completion, including audio processing. A Session produces one Note. |

**Architectural note:** Record and Dictate are the same recording capability under the hood — same audio technology, same Whisper transcription. The differences are capture configuration (audio durability, model quality tier, stop safeguards, activation method, output destination). The current two-controller architecture (`ScribeController`, `DictateController`) is an artefact of how the app evolved, not a domain distinction. Future refactoring may unify them.

---

## App navigation & UI taxonomy

The full UI taxonomy lives in `ui-taxonomy.md`. Summary of levels relevant to app structure:

| Taxonomy level | Definition | Examples in ScribeFloat |
|---|---|---|
| **Token** | A single named design value | colour, spacing, radius values |
| **Primitive** | Structural or display building block, not used standalone | `ScrollBody`, `PanelHeader`, `StatusDot` |
| **Component** | A single, indivisible user action | `Button`, `NoteCard`, `Toggle` |
| **Pattern** | Multiple components working together as one action | Note triage flow, recording controls |
| **Section** | A contained mental model — about one clearly-named thing | Note detail, Filter panel, Settings group |
| **Region** | A fixed structural area of the layout, regardless of content | Sidebar, title bar, main content area |

**App Areas** (top-level routed Regions):

| Area | Purpose |
|---|---|
| **App** | The single persistent window. Sidebar Region + title bar Region + content Region. Previously called "Shell" — too technical. |
| **Home** | Summary landing Area. Recent Notes, stats, quick actions, Triage inbox. Previously called "Dashboard." |
| **Notes** | Browse all Notes. Filterable by tags, Layer Items, Triage status, capture method. |
| **Upload** | Bulk Note creation from external sources: audio files, markdown files, URLs, video URLs. |
| **Float** | Build and manage Layers, Steps, Flows, Vocabulary. |
| **Settings** | Config, models, permissions, hotkeys. |

Record is not a sidebar Area — accessed via a persistent "New Note" action in the title bar. Dictate is a persistent hotkey-triggered action in the title bar, available from any Area.

---

## Float (AI enrichment) domain

| Term | Definition |
|---|---|
| **Layer** | A named extraction type (e.g. Tags, Keywords, Decisions). Owns a Vocabulary, render type, and schema options. |
| **Item** | A vocabulary entry belonging to a Layer. Name + optional description. Shared across all Notes for that Layer. |
| **Vocabulary** | The shared unique Item list belonging to a Layer, accumulated over time via approvals. |
| **Step** | A single extraction instruction: targets one Layer, carries a prompt and chunk strategy. Reusable across Flows. |
| **Flow** | An ordered sequence of Steps with a trigger (`on-creation` or `manual`). Running a Flow on a Note produces a Result. |
| **Result** | The output of a Flow run on one Note. Has a status: `draft` → `edited` → `approved`. A `draft` Result is in Triage. On Approve, new Items are promoted into the Layer's Vocabulary. |
| **Agent Action** | Any decision or change made by Float on a Note — adding tags, extracting decisions, editing the Note Body, suggesting archival, renaming the title. Every Agent Action produces a Result that enters Triage. |

---

## Triage

| Term | Definition |
|---|---|
| **Triage** | The review queue for Notes with pending Agent Actions. Triage is per-Note — you review the Note's full pending state in one pass, not individual Flows or Steps. |
| **Pending** | A Note that has un-triaged Agent Actions. Multiple Float runs on the same un-triaged Note merge into one Triage item — the user deals with everything at once. |
| **Approved** | The user has signed off on a Note's pending state. Agent Actions become permanent. New Items are promoted into the Layer's Vocabulary. |
| **Rejected** | The user dismissed a pending action on a Note. That Agent Action is discarded. |

**The rules:**
- Triage is per-Note, not per-Flow-run. You triage a Note once.
- If multiple Flows run on a Note before it's triaged, their Results merge into one Triage item.
- Once a Note has been triaged, subsequent Float runs on it are applied directly — no second Triage cycle.
- Triage is universal: it is not specific to any capture method.

**Where Triage surfaces** — it is not a separate Area:
- **Home**: primary triage surface, the inbox. Where the user actions pending Notes.
- **Notes Area**: filterable by Triage status so the user can find pending Notes.
- **Note view**: inline triage — the Note's status is visible and the user can approve or reject without leaving the Note.

---

## Deprecated terms

| Deprecated | Replaced by | Reason |
|---|---|---|
| ~~History Record~~ | **Note** | Too technical; doesn't reflect the user's mental model |
| ~~Shell~~ | **App** | Too technical |
| ~~Dashboard~~ | **Home** | "Home" is a more natural user destination |
| ~~Transcribe~~ (as a workflow name) | **Upload** | Describes the user action, not the backend process |
| ~~Scribe~~ | **Record** | "Record" is a verb like Dictate and Upload; "Scribe" was a product-era name that leaked into the domain |
| ~~NotePanel / NoteComposer / NoteCard~~ | Markdown text area in Note Body | Chat-style note taking deprecated; replaced by a unified editable Note Body |

---

## Knowledge layer

The knowledge layer is a future phase that builds on Float's vocabulary. It is not yet built.

| Term | Definition |
|---|---|
| **Knowledge** | The overall knowledge system in ScribeFloat. The layer above Notes — where synthesized, cross-Note understanding lives. |
| **Domain** | A named knowledge area within Knowledge (e.g. "ScribeFloat Vision", "UX Practice", "Product X"). A folder. Contains Artifacts and can link to other Domains. |
| **Artifact** | A synthesized knowledge document within a Domain. Built from many Notes over time. Has a type (persona, stakeholder, user flow, decision log, user quotes, etc.) expressed as frontmatter. A markdown file — human- and agent-readable, no database. |
| **Artifact type** | What kind of Artifact it is. Determines structure and fields. Fixed catalog to start; user-extensible later. |

**Information hierarchy:**

```
Knowledge
  └── Domain          e.g. "ScribeFloat Vision", "UX Practice", "Product X"
        └── Artifact  e.g. persona, decision log, user flow, stakeholder
              ↑
              linked via tags & keywords (Float Vocabulary)
              ↑
Notes  →  Sources     transcript, written, web, upload_audio...
```

Notes are linked to Artifacts through Float's tag and keyword Vocabulary — Float tags a Note, that Note becomes source material for any Artifact in a Domain that shares those tags. Artifacts reference back to the Notes they were built from.

Storage: Domains are folders. Artifacts are markdown files with YAML frontmatter (`type:`, `title:`, `tags:`, `sources:`). Human-readable, agent-readable, diffable in git. No database required.

---

## Open questions

None.
