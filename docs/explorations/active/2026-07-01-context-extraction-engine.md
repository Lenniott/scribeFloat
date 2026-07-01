# Context Extraction Engine — reconciling Broccoli with Float

> Status: **active exploration**. Reconciles the `tools/project_package` ("Broccoli") memory
> experiment with `design-brain-prd.md` (Float engine) and `knowledge-orchestration.md` (the
> tag-and-export pivot). Nothing here is built. The extraction *recipe* (categories, prompts,
> chunking) is expected to keep changing — this doc fixes the *architecture* around that recipe,
> not the recipe itself.
>
> Prerequisite reading: [`design-brain-prd.md`](design-brain-prd.md),
> [`knowledge-orchestration.md`](knowledge-orchestration.md),
> `tools/project_package/docs/PRODUCT_LEARNINGS.md`

---

## 1. What already exists (don't re-derive, reuse)

| Piece | State | Where |
|---|---|---|
| HTTP inference client (Ollama / OpenAI / Anthropic) | **Built** | `src-tauri/src/services/inference.rs` |
| Float config (provider, endpoint, key, model) | **Built** | `Config` fields in `src-tauri/src/types.rs`, `FloatController` |
| Enrichment queue, flow engine, chunker, layer registry | **Not built** | described in `design-brain-prd.md` §7.2 |
| Tags (Layer/Step/Flow, MVP scope) | **Not built**, spec frozen | `design-brain-prd.md` |
| Tag + annotate + on-demand export | **Design pivot agreed, not built** | `knowledge-orchestration.md` "Pivot" section |
| Thread-memory extraction + embeddings + pack briefs | **Python prototype only, product learning captured** | `tools/project_package` |

The three design docs disagree on one thing: whether embeddings/vector storage are needed.
`design-brain-prd.md` deferred them (no use case yet). `knowledge-orchestration.md`'s pivot
rejected them again in favour of deterministic grep+timestamp export. Broccoli's experiment is
the first real evidence either way — and its finding is that **lexical grep alone can't answer
open-ended questions like "what workflows could we automate here"** — that requires similarity
search over meaning, not just tag/keyword overlap. This doc treats that as settled: embeddings
are in, scoped to what Broccoli actually needed, not a general-purpose vector DB.

---

## 2. Two extraction tiers, one runner — restated

Nothing here changes the two-flow shape already agreed:

- **Flow 1 — Tags.** Lightweight, always-on, per-note vocabulary. Unchanged from
  `design-brain-prd.md`. Feeds routing/filtering signal. Requires user approval before its
  vocabulary is trusted.
- **Flow 2 — Context extraction (this doc).** Heavier, runs on every note automatically
  (no approval gate — see §4), produces the thread-memory objects Broccoli prototyped, and is
  the *only* place embeddings are used.

Both are steps of the same underlying HTTP runner (`InferenceService`). Flow 2 does not depend on
Flow 1 having been approved — unlike the earlier 12-type knowledge-extraction design, thread
memory isn't routed to a domain and isn't shared vocabulary, so it carries none of the "don't
compound errors on unapproved tags" risk that motivated gating Flow 2b behind Tags approval.

---

## 3. Context Configs — the tunable unit

This is the direct answer to "we need to keep tweaking how we process transcripts." Everything
Broccoli hardcodes (`CATEGORIES`, `build_batch_prompt`, `build_memory_prompt`,
`PACK_SIGNAL_LEXICON`, `OPPORTUNITY_LABELS`) becomes **data, not code** — a stored, user-editable
object:

```json
{
  "config_id": "thread-memory-v3",
  "version": 3,
  "categories": ["Situation", "Problem", "Intent", "Option", "Decision", "Evidence", "Open Thread"],
  "batch_prompt": "... templated, {categories} and {unit_text} interpolated ...",
  "synthesis_prompt": "... templated ...",
  "chunk_strategy": { "unit": "segment_boundary", "batch_size": 120, "overlap": 12 },
  "model_ref": "float_config",
  "embed": true,
  "embedding_model": "embeddinggemma:latest"
}
```

Notes:

- **`OPPORTUNITY_LABELS` and `PACK_SIGNAL_LEXICON` do not belong in a Context Config.** Those are
  task-specific to the Sedgwick experiment (tax/planning/SEO labels) — hardcoding them was exactly
  the "ingestion role-specific" mistake `AGENT_CONTEXT.md` already flags as a lesson learned. They
  stay out of the shipped default and are only ever something a *pack brief* supplies at retrieval
  time (§5), never baked into extraction.
- **Chunking should align to Whisper segment boundaries**, not Broccoli's sentence-regex
  unitisation — the app already has real segment timestamps; reuse them, don't re-derive sentence
  boundaries from scratch.
- **Versioning is informational, not migration-safe.** Bump `version` when the recipe changes;
  each per-note memory file records which `config_id`/`version` produced it. Because this is
  single-user, there is no requirement to keep old and new outputs compatible — re-running a note
  under a new config simply replaces its memory file. No migration code needed.
- **Authoring UX** follows the existing Float pattern (`design-brain-prd.md` §6.3): a form for the
  few structural knobs (chunk size, embed toggle) plus a plain textarea for the two prompts.
  "Describe it, LLM scaffolds it" can come later; a working textarea is the MVP.
- A Context Config can be **re-run on demand** — one note, a filtered set, or the whole corpus —
  independent of when it last ran automatically on creation.

---

## 4. The cue — triggering extraction

Same mechanism `design-brain-prd.md` already designed for Tags, extended to a second automatic
job on the same queue:

```
Recording finishes → Whisper transcribes → HistoryService.append (DONE)
  → EnrichmentQueue.enqueue([tags_flow, context_extraction_flow])
  → worker (concurrency 1) runs Tags, then Context Extraction, sequentially
  → memory file written per note, status implicit (no draft/approved — see below)
```

Context extraction does **not** get a draft/approved review step. Broccoli's `confidence` field
(low/medium/high, degrading to a labelled fallback extraction when the model output is unusable)
already carries the trust signal, and forcing per-thread triage on every note would recreate the
"data-entry-first" friction the target user (ADHD, doesn't file things) will abandon. Review
happens implicitly, at read time: a low-confidence or fallback-extracted thread is visibly marked
as such wherever it surfaces (per-note memory view, retrieved pack). This matches
`knowledge-orchestration.md`'s already-agreed principle that tag annotation and export are
deterministic/no-hallucination-risk at read time — the LLM work happens once, at capture, and its
confidence travels with it rather than being re-verified by the user before it's usable.

---

## 5. Storage — cutting the duplication

The duplication problem you flagged is real and traceable to one thing in the prototype: full
`source_text` is copied onto every occurrence, then `source_excerpt` copies it again (trimmed),
then `embedding_text` on the index row concatenates copies of the same summaries a third time.
None of that needs to exist as stored text — it can be resolved lazily.

**Per-note memory file** (lives beside the note, same pattern as `note_sidecar.rs`):

```json
{
  "note_id": "...",
  "source_hash": "...",
  "config_id": "thread-memory-v3",
  "threads": [
    {
      "memory_id": "mem_...",
      "line_of_inquiry": "...",
      "summary": "...",
      "confidence": "medium",
      "occurrences": [
        { "segment_start": 41, "segment_end": 44, "category": "Decision", "summary": "..." }
      ]
    }
  ]
}
```

Occurrences store **segment refs (or char offsets), not source text.** The transcript already
exists in the note; a 100–150 char excerpt is generated on demand when something needs to display
one (pack assembly, per-note view) by slicing the real transcript at those offsets — never stored
a second time. This alone removes the largest duplication source in the prototype.

**Global memory index** (`float/memory_index.jsonl`) — one denormalized row per thread for fast
scanning across the corpus: `memory_id, note_id, line_of_inquiry, summary, confidence,
category_counts`, plus a pointer back to the per-note file. No occurrences, no source text here —
this file is a scan index, not a store of record.

**Embeddings** — kept in their own file, as Broccoli already correctly does (`memory_embeddings.jsonl`
separate from `memories.jsonl`), because you rarely want to load 3–4 KB of floats just to scan
summaries. Two changes from the prototype:

- Store the embedding as **base64-encoded float32**, not a JSON array of floats — roughly 3–4x
  smaller and cheaper to parse than `[0.0123, -0.0456, ...]` repeated per dimension.
- Keep `embedding_hash` (hash of the text that was embedded) exactly as Broccoli does, to skip
  re-embedding unchanged threads — this dedup logic is already correct, keep it as-is.

**On vector DB / ANN indexing:** not needed. This reaffirms the existing ADR reasoning
(`design-brain-prd.md` §2 — "one user's personal history... thousands of records, never
millions") — brute-force cosine over an in-memory float matrix is fast enough at this scale and
needs no server process, matching the local-first constraint. Revisit only if a single user's
corpus provably reaches tens of thousands of threads and a scan becomes measurably slow — not
before.

**Cross-note duplicate threads** (the same recurring topic appearing in many notes) are
deliberately **not merged at storage time** — merging identity is a hard, potentially destructive
problem (`knowledge-layer-intent.md` already flagged synthesis as a separate, later, user-triggered
phase). Instead, retrieval-time grouping (Broccoli's `opportunity_label` bucketing) gives the
appearance of one concept without an irreversible merge. Storage stays append-only and safe.

---

## 6. Retrieval and final output — the Context Pack

This is unchanged in shape from Broccoli's validated design, and it **is** the deliverable the
user actually gets:

1. User supplies a lightweight brief: `request` (one sentence) + optional `tags`/`signals`.
2. System expands the brief into retrieval signals (tags → signal lexicon → query text). This
   expansion logic is itself something a Context Config can own, so it's tunable too — but the
   *lexicon content* (tax/SEO/etc.) is never shipped as a default; only the mechanism is.
3. Retrieval requires a relevance gate, not nearest-neighbour alone — Broccoli's own postmortem
   (`AGENT_CONTEXT.md` "What Went Wrong") is the evidence: pure embedding ranking surfaced
   unrelated memories on the first pack attempt. Combine cosine score with a lexical
   matched-terms-≥2 gate before a memory is eligible for selection at all.
4. Assembly is **deterministic** — no LLM call at pack-generation time, consistent with
   `knowledge-orchestration.md`'s already-agreed "fast, no hallucination risk" principle. The LLM
   work happened once, at capture.
5. Empty state is explicit and mandatory: "No sufficiently relevant memories found" beats a
   plausible-but-wrong pack every time — this is a hard product rule from the experiment, not a
   nice-to-have.
6. Output: one markdown file, source-linked (segment refs resolved to excerpts at write time),
   saved to `knowledge/exports/` with a datestamp — matching the storage location
   `knowledge-orchestration.md` already settled on for context exports. Not a maintained artifact;
   the user grabs it and takes it to their AI tool of choice.

This is the **final output for the user**: not the per-note memory file (that's internal/debug,
optionally viewable on the Note detail screen for trust-building), but the Context Pack — a single
markdown handoff document produced on demand from a one-sentence ask.

---

## 7. CLI — terminal agent access without a proprietary layer

Already anticipated in `knowledge-orchestration.md`'s "Further simplifications" section; this
extends it to cover the memory/pack layer specifically:

```bash
scribefloat memory list [--note <id>]
scribefloat memory rebuild --config thread-memory-v3 [--note <id> | --all]
scribefloat pack --request "..." [--tags ...] [--signals ...] [--max-memories N]
scribefloat context-config list | show <id> | edit <id>
scribefloat tags list
scribefloat notes read <id>
```

**Shape:** extract the extraction/retrieval/pack-assembly logic (queue, chunker, memory store,
pack builder, inference client) into a plain Rust library crate with no Tauri dependency. The
Tauri app and a separate `scribefloat` CLI binary both depend on that library and operate directly
on the same on-disk save folder — no daemon, no IPC socket. This is consistent with the
local-first, file-based state the rest of the app already commits to (`history.jsonl`,
`note_sidecar.rs`, markdown knowledge exports) — the CLI is just a second front door onto the same
files, not a new service. A terminal coding agent gets `scribefloat pack --request "..."` instead
of needing to be taught the extraction/retrieval logic itself.

---

## 8. Non-goals (explicit, not forgotten)

- Cross-note thread merging / persona-style synthesis — deferred to the knowledge-artifact idea
  (`knowledge-layer-intent.md`), which stays a later, user-triggered phase.
- Draft/approve triage on context-extraction threads — deliberately skipped (§4).
- Any hosted/server-side vector index — brute-force local file is sufficient at this scale (§5).
- Domain/type routing (the 12-type knowledge agent design) — fully superseded by the pivot in
  `knowledge-orchestration.md`; this doc does not resurrect it.
- Shipping any task-specific label/lexicon content (tax, SEO, workflow-automation labels) as
  defaults — those were experiment-specific and stay out of the product surface.

---

## 9. Open questions

1. Do Tags (Flow 1) and Context Extraction (Flow 2) run as one enqueued job or two separately
   queued jobs? Sequential-in-one-job is simpler; separate jobs let one be disabled without
   touching the other.
2. Where exactly does the per-note memory file live — inside the note's own folder (alongside
   `note_sidecar.rs` output) or in a parallel `float/memory/<note_id>.json` tree? Affects how it
   travels if a note folder is exported/moved.
3. Segment-boundary chunking assumes Whisper segments are the natural unit — does this hold for
   `written` and `upload_audio`/`import_md` Sources that have no Whisper segments? Likely falls
   back to paragraph/sentence chunking for those Source types.
4. Should `context-config edit` be exposed in the GUI at all in the first cut, or CLI/file-edit
   only while the recipe is still churning? Given this is still active research, a raw editable
   JSON/TOML file (no UI) may be the right MVP — build the UI once the recipe stabilises.

---

## 10. Conversation Analysis signals — exploratory, not yet decided

> Surfaced from a study of Conversation Analysis / Conversational UX (Harvey Sacks, Bob Moore).
> These are hypotheses to test in the next extraction-quality pass (`PRODUCT_LEARNINGS.md`'s
> "next experiment is quality work"), not committed design. Marked here so the reasoning survives
> past the conversation that produced it.

### 10.1 A second classification axis: Activity, alongside Aspect

The 7 aspects (§3) classify *what kind of content* an occurrence is (Decision, Problem, ...).
They say nothing about *what shape of interaction* it's embedded in. Conversation Analysis'
"Pattern Language" (~100 generic interaction shapes — troubleshooting, instruction-giving,
extended telling, quiz) is a cheap second field to add per occurrence or segment, orthogonal to
aspect. Use: a segment classified as "Instruction Giving" is a candidate for a workflow-diagram
output template; a "Quiz" segment isn't. Not yet designed further than this.

**Considered and rejected — Slots.** Task-oriented slot-filling ("for an Instruction-Giving
activity, extract slots X/Y/Z") was also proposed. Rejected: it's a fixed schema authored ahead of
time, which is exactly the anti-pattern `AGENT_CONTEXT.md` already burned itself on — "do not make
users define fixed recipes." If structured parameters are wanted later, they should come from a
small number of universal activity templates, not user-authored slot schemas.

### 10.2 Chunking refinement: gap vs. pause as a Transition Relevance Place signal

Refines `chunk_strategy` in §3, doesn't replace it. Whisper's timing already distinguishes a
comfortable inter-turn *gap* from an intra-turn *pause* — that distinction is a sharper signal for
where one thought ends and another begins than a fixed `batch_size_units` count. Worth testing
against segment-boundary chunking as-is before assuming it's an improvement.

### 10.3 Preference organization as an Option-vs-Decision confidence signal

Dispreferred responses (rejection, disagreement) are typically hedged, delayed, and longer than
preferred ones (quick uptake). Usable as a heuristic feature in the batch prompt to help the model
distinguish "this was floated and dropped" (Option) from "this was actually agreed" (Decision)
with more than its own unaided judgment.

### 10.4 Compression, the accordion model, and a rehydration pass

The core finding from this round of discussion. Recipient Design + Minimization mean speakers
compress based on their estimate of what the listener already shares — "the usual pipeline,"
a bare name, an acronym — and that compression is invisible in the transcript itself. Nothing in
the pipeline as designed so far (aspects, threads, embeddings, pack assembly) accounts for this,
and it matters directly for this product because recurring stakeholder relationships are the
premise, not the exception.

**Resolution order for a flagged compressed reference, cheapest first:**

0. **Scan backward within the same transcript.** The presence of compression is itself evidence
   that a richer, establishing version of the same referent exists somewhere earlier — not
   necessarily adjacent. Cheapest possible check: same source, no retrieval needed.
1. **Check for an adjacent repair sequence.** Recipient signals confusion ("what do you mean,"
   "who?") in third position, speaker abandons compression and gives the "longer, simpler,
   clunkier" unpacked version. When present, this is the single best source of rich context
   available — a human already explained it, nothing was inferred.
2. **Cross-note corpus retrieval** — only if 0 and 1 find nothing (the accordion never opened;
   two experts, no confusion, no local record of the fuller meaning). Reuses the same
   embedding + lexical-gate retrieval mechanism already designed for pack assembly (§6), scoped by
   whatever tags the note already carries — degrades gracefully if Tags aren't approved yet rather
   than blocking on approval.
3. **Leave unresolved, flagged, no invention.** If nothing is found, mark
   `external_reference: unresolved` rather than letting the model synthesize a plausible-sounding
   definition for jargon it has no legitimate way to know. Same rule as the pack empty-state (§6):
   no confident answer beats a wrong one.

### 10.5 Compression as the wheat/chaff and dedup mechanism

This is the sharpest product implication, and it answers the original duplication concern more
directly than the storage changes in §5:

- An occurrence showing **heavy compression** is evidence the thread was already established
  elsewhere in this transcript or a prior one — it adds no new information. Store it as a
  lightweight recurrence pointer only (segment ref, no full excerpt, no embedding spent on it).
- An occurrence showing **unpacking/expansion** is the canonical, information-rich instance of the
  thread — this is the one that gets a full excerpt, an embedding, and is what the thread's stored
  `summary` is actually built from.

This is cheaper than embedding-similarity-based dedup (cluster everything, then collapse
near-duplicates after the fact): compression is detectable directly from the text before any
embedding is computed, so it decides what's worth embedding rather than embedding everything and
pruning afterward.

Worth a new per-occurrence marker, separate from `confidence`: something like
`self_contained: true/false` — richness is about how much external context a reader would need,
not about how sure the extraction is.

**Free glossary source, as a side effect.** A repair-triggered unpacking sequence is a candidate
definition entry, sourced from what a human actually said rather than a model-authored gloss —
a much lower-risk path toward something like the Glossary type the earlier 12-type knowledge
design carried (deferred, not abandoned, per `knowledge-orchestration.md`), without reviving that
architecture.

### 10.6 Context Pack assembly rule: surface only the uncompressed form

Direct consequence for §6 step 6 (assembly): when a thread has multiple backing occurrences, the
assembled pack should quote the least-compressed one — the richest, most self-contained version —
never the first-seen or most-recent one by default. Compressed occurrences count toward "this
recurred N times" but are never what gets quoted.

### 10.7 Recipient Design applied to pack *output* (smaller, separate idea)

Context Packs are currently scoped by topic only (`request`/`tags`/`signals`, §6). Recipient
Design suggests they could also take an audience parameter — the same underlying facts phrased
differently for a stakeholder briefing vs. handoff to an AI model with no shared background
(UC-6, `accumulated-context-problem.md`). Noted, not designed.

### 10.8 New open questions from this section

- How is "compression" actually detected in practice — regex/heuristic markers (bare definite
  reference, acronym, undefined proper noun), or a cheap classification pass? Needs empirical
  testing against real transcripts before committing either way.
- Does the backward in-transcript scan (§10.4 tier 0) need its own lightweight similarity check,
  or can it reuse the same per-note occurrence list directly without any embedding at all?

---

## 11. Reference

| Document | Relationship |
|---|---|
| [`design-brain-prd.md`](design-brain-prd.md) | Float engine, Tags (Flow 1), HTTP runner, queue design this reuses |
| [`knowledge-orchestration.md`](knowledge-orchestration.md) | Tag+annotate+export pivot; context pack storage location; CLI precedent |
| [`knowledge-layer-intent.md`](knowledge-layer-intent.md) | Cross-note synthesis (personas, project arcs) — later phase, not this one |
| `tools/project_package/` | Source experiment this doc reconciles into the app architecture |
| `docs/backlog/active/0052-float-inference-backend.md` | Current build status of the HTTP runner this reuses |
