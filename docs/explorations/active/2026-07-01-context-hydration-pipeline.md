# Context Hydration Pipeline — chunks, blocks, and packs

> Status: **active exploration — current design**. Replaces the compression/rehydration
> exploration in `docs/explorations/superseded/2026-07-01-context-extraction-engine-v1.md`
> (§10.1-10.9 there) with a simpler, more direct mechanism reached in conversation. Carries
> forward that doc's still-valid infrastructure decisions (Context Configs, the cue, storage
> shape, CLI). Nothing here is built. The extraction recipe is expected to keep changing — this
> doc fixes the architecture around it, not the recipe itself.
>
> Superseded predecessors, kept for their reasoning trails:
> [`design-brain-prd.md`](../superseded/design-brain-prd.md),
> [`knowledge-layer-intent.md`](../superseded/knowledge-layer-intent.md),
> [`knowledge-orchestration.md`](../superseded/knowledge-orchestration.md),
> [`2026-07-01-context-extraction-engine-v1.md`](../superseded/2026-07-01-context-extraction-engine-v1.md)

---

## 1. The core idea

A Context Pack is a chunk-retrieval problem. What makes a chunk worth retrieving isn't how
relevant it is to the request — it's how **self-contained** it is. A human in the original
conversation could decompress "the usual pipeline" because they shared the background. An LLM
reading a pack cold has none of that. A chunk that's on-topic but compressed is closer to noise
than signal once handed to a model with zero prior context.

So the pipeline is built around three units, not one:

- **Chunk** — the smallest extracted unit of meaning (a segment or small group of segments from a
  transcript, or a paragraph from written text).
- **Block** — the smallest *self-contained* unit: either one hydrated chunk on its own, or a
  compressed chunk combined with whichever other chunk supplies what it's missing.
- **Pack** — the deliverable: a set of blocks assembled in response to a one-sentence user request.

A pack is never made of raw chunks. It's always made of confirmed-complete blocks.

---

## 2. What already exists (don't re-derive, reuse)

| Piece | State | Where |
|---|---|---|
| HTTP inference client (Ollama / OpenAI / Anthropic) | **Built** | `src-tauri/src/services/inference.rs` |
| Float config (provider, endpoint, key, model) | **Built** | `Config` fields in `src-tauri/src/types.rs`, `FloatController` |
| Enrichment queue, flow engine, chunker, layer registry | **Not built** | described in `design-brain-prd.md` §7.2 |
| Tags (Layer/Step/Flow, MVP scope) | **Not built**, spec frozen, unaffected by this doc | `design-brain-prd.md` |
| Thread-memory extraction + embeddings + pack briefs | **Python prototype, product learning captured** | `tools/project_package` |

---

## 3. Chunk extraction

**Chunking method is source-dependent** — this was flagged as an open question before the
hydration model existed and stays true:

- Audio-sourced transcripts chunk on turn-taking cues: silence that's unusually long *for that
  conversation's own rhythm* (not a fixed threshold — fluent exchange keeps gaps near-zero because
  speakers anticipate the end of a turn before it arrives), speaker overlap, and wrap-up words
  ("so yeah," "anyway") as a text-visible stand-in for a vocal turn-yielding cue we don't have
  access to from a transcript.
- Written sources (`written`, future `web`/`import_md`) have no turn-taking signal at all — they
  chunk on structural boundaries: paragraphs, headings, list items.

**Each chunk gets one bounded model call** producing three things, not just a category:

```json
{
  "category": "Decision",
  "summary": "...",
  "unresolved": ["the usual pipeline"],
  "defines": []
}
```

- `category` — same seven aspects as before (Situation, Problem, Intent, Option, Decision,
  Evidence, Open Thread). Unchanged.
- `unresolved` — things this chunk's meaning depends on that aren't explained within it. Empty if
  the chunk is already self-contained.
- `defines` — things this chunk itself explains or spells out, if anything (a proactive gloss, a
  definition, an unpacked explanation of something vague).

This directly asks the two facts needed to build blocks, instead of inferring them indirectly
(the superseded doc tried regex compression markers, then embedding-similarity to
clarification-exemplar phrases, then subsumption ranking — all proxies for a question that can
just be asked outright).

---

## 4. Block assembly

- A chunk with an **empty `unresolved` list** is already a complete block on its own — no
  combining needed.
- A chunk with something in `unresolved` needs a partner. Search for candidate chunks (same
  embedding + lexical-overlap retrieval already used for pack assembly, §6) whose `defines` list
  plausibly answers what's missing — matching two short lists against each other is a much more
  precise pairing than generic topic similarity, because it's asking "does this specifically
  answer what that specifically lacks," not just "are these about the same thing."
- **Confirm, don't assume.** Combine the candidate and the original chunk, re-run the same
  self-containedness check on the combined text. Only treat it as a resolved block if the check
  now comes back clean.
- If no candidate resolves it, the block is the chunk alone, **marked incomplete** — never
  invent a resolution. Same rule as the pack empty-state: no confident answer beats a wrong one.
- `unresolved` can, in principle, need more than one partner. Untested — see §9.

**Written self-notes are the hardest case.** A note typed to oneself assumes an audience of
one (future-self) who's assumed to already know everything — maximal compression, and there's no
possibility of an in-document repair ever occurring (no other party to trigger one). These
depend on cross-note retrieval far more often than transcripts do, or stay incomplete more often.

---

## 5. Storage — this is what solves the duplication problem

Storage happens at the **block** level, not the chunk level. A block's full excerpt and embedding
are stored once, computed from its combined (and confirmed self-contained) text. Chunks that got
absorbed into a block don't get separate storage beyond a reference back to their source segment
range — there is no duplicate copy of "the compressed mention" sitting alongside "the full
explanation" once they've been combined into one block.

**Per-note memory file** (lives beside the note, same pattern as `note_sidecar.rs`):

```json
{
  "note_id": "...",
  "source_hash": "...",
  "config_id": "hydration-v1",
  "blocks": [
    {
      "block_id": "blk_...",
      "category": "Decision",
      "summary": "...",
      "self_contained": true,
      "chunk_refs": [
        { "segment_start": 41, "segment_end": 44 },
        { "segment_start": 12, "segment_end": 13 }
      ]
    }
  ]
}
```

`chunk_refs` store segment/offset ranges, not source text — an excerpt is generated on demand by
slicing the real transcript, exactly as before.

**Global index** (`float/memory_index.jsonl`) — one denormalized row per block for cross-corpus
scanning: `block_id, note_id, category, summary, self_contained, category_counts`, plus a pointer
to the per-note file. No chunk text here.

**Embeddings** — kept in their own file (`memory_embeddings.jsonl`), one per block, computed from
the block's combined text. Stored as base64-encoded float32, not JSON float arrays (~3-4x smaller,
cheaper to parse). `embedding_hash` (hash of the embedded text) skips re-embedding unchanged
blocks.

**No vector DB.** Brute-force cosine over an in-memory float matrix is fast enough at "one
person's history, thousands of blocks, never millions" scale, and needs no server process —
matching the local-first constraint. Revisit only if a corpus provably reaches tens of thousands
of blocks and a scan becomes measurably slow.

**Cross-note duplicate blocks** (the same recurring topic appearing in many notes) are
deliberately not merged at storage time — that's a harder, potentially destructive identity
problem, deferred to the knowledge-artifact idea (`knowledge-layer-intent.md`) as a later,
user-triggered phase. Storage stays append-only and safe.

---

## 6. Retrieval and the Context Pack

Unchanged in shape from the prototype's validated design — this is the deliverable:

1. User supplies a lightweight brief: `request` (one sentence) + optional `tags`/`signals`.
2. System expands the brief into retrieval signals. The expansion *mechanism* is tunable via
   Context Config; task-specific lexicon content is never shipped as a default.
3. Retrieval requires a relevance gate, not nearest-neighbour alone — combine cosine score with a
   lexical matched-terms gate before a block is eligible for selection.
4. Assembly is deterministic — no model call at pack-generation time. The model work already
   happened once, at capture (chunk extraction) and at block assembly (confirmation checks).
5. Empty state is explicit and mandatory: "No sufficiently relevant blocks found" beats a
   plausible-but-wrong pack every time.
6. Output: one markdown file, source-linked (chunk refs resolved to excerpts at write time),
   saved to `knowledge/exports/` with a datestamp. Not a maintained artifact — the user grabs it
   and takes it elsewhere.

Because every selected block was already confirmed self-contained at assembly time (§4), pack
assembly never has to re-decide "is this quotable on its own" — that question was answered once,
when the block was built, not re-litigated on every retrieval.

---

## 7. Context Configs — the tunable unit

Everything about the recipe is data, not code:

```json
{
  "config_id": "hydration-v1",
  "version": 1,
  "categories": ["Situation", "Problem", "Intent", "Option", "Decision", "Evidence", "Open Thread"],
  "chunk_prompt": "... templated, asks for category + unresolved + defines ...",
  "block_confirmation_prompt": "... templated, re-checks a combined chunk pair ...",
  "chunk_strategy": { "audio": "turn_taking_aware", "written": "structural" },
  "model_ref": "float_config",
  "embed": true,
  "embedding_model": "embeddinggemma:latest"
}
```

- Task-specific label/lexicon content (tax, SEO, workflow-automation buckets — artifacts of the
  Broccoli experiment's specific test data) never ships as a default; only the mechanism does.
- Versioning is informational, not migration-safe. Bump `version` when the recipe changes; a
  per-note memory file records which config produced it. Single-user, so re-running a note under
  a new config just replaces its memory file — no migration code needed.
- A raw editable JSON/TOML file (CLI-opened) is the right MVP while the recipe is still churning —
  build a GUI once it stabilises, not before.

---

## 8. The cue, and CLI — unchanged from the predecessor doc

**Trigger:**

```
Recording finishes → Whisper transcribes → HistoryService.append (DONE)
  → EnrichmentQueue.enqueue([tags_flow, hydration_flow])  — two separate queue entries
  → worker (concurrency 1) runs each when free
  → per-note memory file written; no draft/approved gate on this flow
```

No review gate on hydration blocks, same reasoning as before: forcing per-block triage on every
note recreates the data-entry friction the target user will abandon. A block's `self_contained`
flag and the model's own confidence travel with it and surface wherever it's shown, rather than
needing user sign-off before it's usable.

**CLI**, so a terminal agent can use this without being taught the extraction logic itself:

```bash
scribefloat memory list [--note <id>]
scribefloat memory rebuild --config hydration-v1 [--note <id> | --all]
scribefloat pack --request "..." [--tags ...] [--signals ...] [--max-blocks N]
scribefloat context-config list | show <id> | edit <id>
scribefloat tags list
scribefloat notes read <id>
```

Shape: extract the extraction/retrieval/pack-assembly logic into a plain Rust library crate with
no Tauri dependency; the Tauri app and a separate `scribefloat` CLI binary both read/write the same
on-disk save folder directly — no daemon, no IPC socket.

---

## 9. Open questions

1. Does the model reliably self-report `unresolved`/`defines` on real transcripts? This is the
   load-bearing assumption of the whole mechanism — needs testing before trusting it.

   **Manual test run (4 hand-written chunks, single-pass holistic prompt):** split result, not a
   clean pass. Genuine shorthand was caught correctly — an unintroduced proper noun ("Acme"), an
   undefined acronym ("QBR"), a vague named reference ("the usual pipeline"). But ordinary
   descriptive/compositional noun phrases were over-flagged as unresolved — "CRM export,"
   "reporting database," "contrast check" — none of which depend on this specific conversation's
   history; they describe themselves through their own words. Adding an explicit instruction
   ("only flag what a domain-competent stranger to *this specific history* couldn't parse") fixed
   one case ("contrast check" dropped) but left the other unchanged ("CRM export"/"reporting
   database" persisted) on a re-run — a holistic "judge whether this is confusing" instruction
   isn't reliably steerable by prompt wording alone.

   **Refined hypothesis, not yet tested:** stop asking for a single holistic judgment. Split into
   two steps — (a) extract candidate phrases (proper nouns, acronyms, definite "the X"
   references) rather than asking for a vague-things judgment directly, then (b) ask a targeted
   yes/no question per candidate: "using only general knowledge, independent of this
   conversation, can you say what '{phrase}' refers to or means?" A "no" becomes `unresolved`.
   This decomposes an unreliable single judgment into a checkable per-item test. Known edge case
   to watch for: some acronyms are generically definable in the abstract ("QBR" = quarterly
   business review is common knowledge) even though the *specific* reference ("the QBR on
   Friday," whose) still depends on the conversation — the question likely needs to be asked
   about the reference-in-context, not the bare term in isolation.

   **Also confirmed:** chunk-to-chunk matching must be semantic, not exact string equality. One
   test chunk's `defines` said "nightly sync job"; a separate chunk's `unresolved` said "usual
   pipeline" — almost certainly the same referent, worded differently. Literal list comparison
   between `defines` and `unresolved` would never link this pair.
2. Can `unresolved` ever need more than one partner chunk to resolve? If so, block assembly needs
   to be iterative (keep searching until the combined check passes or a budget runs out) rather
   than a single match-and-confirm step.
3. Where does the per-note memory file physically live — inside the note's own folder, or a
   parallel `float/memory/<note_id>.json` tree? Affects portability if a note folder is
   exported/moved.
4. Written self-notes (§4) may need a different default retrieval budget (search further, more
   often) since local resolution is rarely available to them at all.

---

## 10. Non-goals (explicit, not forgotten)

- Cross-note thread/persona merging — deferred to `knowledge-layer-intent.md`, a later,
  user-triggered phase.
- Draft/approve triage on hydration blocks — deliberately skipped (§8).
- Any hosted/server-side vector index — brute-force local file is sufficient at this scale (§5).
- Domain/type routing (the earlier 12-type knowledge agent design) — fully superseded, not
  resurrected here.
- Indirect compression-detection heuristics (regex markers, embedding-similarity-to-exemplar
  phrases, subsumption ranking) — tried in the reasoning trail, replaced by the direct
  `unresolved`/`defines` question because it answers what's needed instead of inferring it.
- Shipping any task-specific label/lexicon content as defaults.

---

## 11. Reference

| Document | Relationship |
|---|---|
| [`../superseded/2026-07-01-context-extraction-engine-v1.md`](../superseded/2026-07-01-context-extraction-engine-v1.md) | Direct predecessor; §1-9 there carry forward mostly unchanged, §10 replaced by this doc |
| [`../superseded/design-brain-prd.md`](../superseded/design-brain-prd.md) | Float engine, Tags (Flow 1, unaffected), HTTP runner adoption rationale |
| [`../superseded/knowledge-orchestration.md`](../superseded/knowledge-orchestration.md) | Tag+annotate+export pivot; CLI precedent; source-linking convention |
| [`../superseded/knowledge-layer-intent.md`](../superseded/knowledge-layer-intent.md) | Cross-note synthesis (personas, project arcs) — later phase, not this one |
| `tools/project_package/` | Source experiment (Broccoli) this lineage reconciles into the app architecture |
| `docs/backlog/active/0052-float-inference-backend.md` | Current build status of the HTTP runner this reuses |
