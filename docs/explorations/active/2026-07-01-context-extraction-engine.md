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

## 10. Reference

| Document | Relationship |
|---|---|
| [`design-brain-prd.md`](design-brain-prd.md) | Float engine, Tags (Flow 1), HTTP runner, queue design this reuses |
| [`knowledge-orchestration.md`](knowledge-orchestration.md) | Tag+annotate+export pivot; context pack storage location; CLI precedent |
| [`knowledge-layer-intent.md`](knowledge-layer-intent.md) | Cross-note synthesis (personas, project arcs) — later phase, not this one |
| `tools/project_package/` | Source experiment this doc reconciles into the app architecture |
| `docs/backlog/active/0052-float-inference-backend.md` | Current build status of the HTTP runner this reuses |
