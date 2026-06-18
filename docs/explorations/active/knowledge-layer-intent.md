# Knowledge Layer — Document of Intent

> Status: **intent only**. Nothing here is scoped or committed. This exists so the idea has a fixed reference rather than living in conversation history.  
> Prerequisite reading: [`design-brain-prd.md`](design-brain-prd.md) (Float enrichment engine).

---

## The problem Float doesn't solve

Float extracts structured data *from* individual transcripts — tags, keywords, decisions. It builds a vocabulary you can filter by. That's an index.

What it doesn't do is *synthesise across* transcripts over time. After 40 user research sessions, you don't have a user persona — you have 40 filtered records. After six months of project recordings, you don't have a project arc — you have a dated list. The knowledge is latent in the corpus; nothing assembles it.

This is the gap the knowledge layer addresses.

---

## What a knowledge artifact is

A persistent document that is **built from many transcripts** and **evolves as new ones arrive**. Not a view over the data — an authored thing you can read, edit, and hand to someone.

Examples in the context of a designer's recording habit:

| Artifact | Built from | Evolves when |
|---|---|---|
| **User persona** | User research recordings tagged with a participant type | A new research session is recorded and approved |
| **Project arc** | All sessions tagged to a project, in order | Any new session for that project is added |
| **Working style** | Personal dictation and reflection recordings | Patterns recur across new sessions |
| **User quotes library** | Any session where a user was present | A new user session is approved |
| **Decision log** | Sessions where Decisions layer was approved | A new decision is added to a project's record |
| **Domain vocabulary** | Keywords layer vocabulary, accumulated over time | New terms are approved into the vocabulary |

The common shape: a document with a structured header (source records, type, last updated) and a free-form body that a person writes, edits, and signs off on.

---

## Relationship to Float

Float is a prerequisite, not a replacement. The two do different jobs:

**Float (extraction):** Per-transcript, bottom-up. Runs prompts against one transcript, produces structured vocabulary (tags, keywords, decisions). Builds the index.

**Knowledge layer (synthesis):** Cross-transcript, top-down. Uses Float's vocabulary to find relevant transcripts, then produces a document artifact. The artifact is the deliverable.

Float's vocabulary tells you which transcripts are relevant to synthesise from. Without it, synthesis has no way to scope its source material — you'd be synthesising across everything.

---

## What makes this hard

Three things that are absent today and would need to exist:

1. **Retrieval.** Synthesising across 30 recordings doesn't fit in one LLM call. You need to select relevant transcript chunks, not whole documents. Float's tag/keyword index is a start; semantic search (embeddings) is the full answer. Deferred until the index proves insufficient.

2. **Artifact storage.** Float stores vocabulary as term lists on `HistoryRecord`. An artifact is a document — different shape, different store, different editing model. Likely: markdown files with structured frontmatter, separate from `history.jsonl`.

3. **Editing.** Approving a vocabulary term is a checkbox. Editing a synthesised persona is a text editor. The UI interaction is fundamentally different — in-app rich text or markdown editing, not a review checklist.

---

## What this is not

- **Not a search engine.** The goal is a document you can hand to someone, not a query interface.
- **Not automatic.** Synthesis requires intent — the user triggers "build a persona from these sessions," not "run automatically on every new recording."
- **Not Float Phase B.** Float Phase B (the Float PRD) activates the enrichment engine and vocabulary workflow. The knowledge layer is a later phase that consumes that vocabulary as input.
- **Not scoped yet.** This document captures the idea. It does not commit to a timeline, an implementation approach, or a UI design.

---

## What needs to be true before this is buildable

In rough dependency order:

1. Float Phase B ships — vocabulary accumulates, tags and keywords are being approved on real transcripts
2. In-app text editor exists — some component that can display and edit a markdown document
3. Artifact store is defined — even a stub data model (`artifacts/` folder + index) that the dashboard can reference
4. Enough vocabulary density — synthesis quality degrades if the corpus is thin; this only becomes useful once a user has meaningful tag/keyword coverage across dozens of sessions

The dashboard can lay groundwork for steps 2 and 3 without Float being live — the editor and the artifact store slot structure are Phase A-compatible additions.

---

## Reference

| Document | Relationship |
|---|---|
| [`design-brain-prd.md`](design-brain-prd.md) | Float enrichment engine — prerequisite; the vocabulary Float builds is the input to synthesis |
| [`dashboard-prd.md`](dashboard-prd.md) | Dashboard shell — the surface where artifacts would eventually live and be accessible |
