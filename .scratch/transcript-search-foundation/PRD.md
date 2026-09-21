# PRD: Fast searchable transcript store

Status: Draft — foundational dependency for Research Regroup
Product: ScribeFloat
Effort: transcript-search-foundation
Date: 2026-09-21

## Need

Every completed transcript must be written into a local database and indexed so a researcher can search across transcripts quickly, open a result in its original conversation, and collect exact source-linked evidence for Regroup. Search must work without Float or a generative model.

This is the foundational capability underneath Find → Collect → Group → Tag → Reflect. Search retrieves passages; it does not create research groups or interpretations.

## User job

When I remember a phrase, topic, or observation from an interview, I want to find the relevant part of my transcripts immediately, see enough context to judge it, and return to the exact source lines without hunting through files.

## Repository constraints / architectural decision required

- ScribeFloat's Note is the primary entity and Transcript is a Source type (CONTEXT.md).
- Binding ADR-0015 defines the Whisper Segment as the transcript atom, with optional speaker, timing and stable array-index pointers after freeze. It describes retrieval chunks as references to Note segments, not copied authoritative transcripts. The line-first migration was still in progress when that ADR was written.
- ADR-0005's markdown storage decision concerns future Knowledge **Artifacts**, not transcript search. Do not use it as an argument against a local transcript search database.
- **Proposed resolution:** persist completed transcript segments into a local queryable database and FTS index as a rebuildable search projection of Note/Source data. The existing Note remains the authoritative source until an explicit ADR changes that contract. If the desired outcome is for the database itself to become the canonical transcript store, resolve that separately before implementation; do not accidentally create two competing sources of truth.

## Functional requirements

### Ingest
1. On successful completion of transcription and speaker alignment, write each Segment to the local database with Note ID, Source ID, stable segment index, text, optional speaker, start/end time, and index version/status.
2. Index automatically when the Note is ready; no manual import step for new recordings.
3. Support backfilling existing Notes, idempotent re-indexing, deletion, and repair after interrupted writes.
4. Keep search results coherent with Note deletion, transcript correction, or segment-array replacement. A replacement that invalidates indexes must rebuild the affected Note/Source.
5. Surface indexing progress and failure; unindexed Notes must not silently appear as complete search coverage.

### Fast lexical search
1. Provide a local full-text search index; SQLite FTS5 is the proposed baseline to prototype, not a committed ADR.
2. Search across all indexed transcripts and optionally scope to a Note, Source, or speaker.
3. Support literal words and phrases, including partial terms where practical; specify tokenisation and punctuation behaviour during implementation.
4. Return ranked results with a short matched snippet, Note identity, speaker/time when available, and source segment indexes.
5. Selecting a result hydrates the original Segment text plus surrounding context from the authoritative Note/Source, with the match visible.
6. A result must be usable for selecting multiple separate evidence ranges; do not force a whole result/chunk into one card.
7. Search and result opening must work entirely offline and without any LLM.

### Data integrity and privacy
- Exact quotations come from stored source segments, not generated paraphrases or an embedding payload.
- Database rows and index entries are derived/rebuildable until the source-of-truth ADR is deliberately revised.
- Handle older Notes lacking speaker labels without excluding them.
- Store and query locally; do not send transcript text to a search service or introduce telemetry.

## Search shape (proposed)

`Note / Transcript Source → completed Segments → local segment table → FTS index → ranked matching segment(s) → context hydration → researcher selects exact evidence`

Illustrative segment row:
`note_id, source_id, segment_index, text, speaker?, start_ms?, end_ms?`

Illustrative result:
`note_id, source_id, matched_segment_indexes, snippet, rank`

Context windows can be assembled from neighbouring indexed segments or existing ADR-0015 retrieval chunk pointers. They are not new authoritative copies of transcript text.

## Performance targets (initial, to validate on target hardware)

- Perceived response should be immediate for normal queries; aim for p95 <= 200 ms from submitted lexical query to first result for a representative local corpus, excluding app cold start.
- Aim for p95 <= 100 ms to open the result and display surrounding transcript lines when the Note is locally available.
- Indexing happens off the interactive UI thread; searching remains responsive while other Notes are being indexed.
- Benchmark realistic corpus sizes (e.g. 10, 100 and 1,000 interview-length Notes) on target desktop hardware. These are proposed acceptance thresholds, not measured results or guarantees.

## MVP acceptance criteria

1. Complete a recording; its transcript becomes searchable automatically once processing finishes.
2. Search an exact phrase from an indexed interview and open the matching source line in context.
3. Search across several Notes and narrow results to one Note; speaker filtering works where speaker metadata exists.
4. Collect two independently selected evidence ranges from one result without losing their exact Note/Source/segment provenance.
5. Restart the app and repeat the search without re-transcribing or rebuilding the entire index.
6. Backfill a pre-existing Note, remove a Note, and update a corrected transcript without stale or orphaned results.
7. Search functions offline, with Float disabled, and meets measured latency targets on the agreed representative corpus.
8. An indexing failure is visible and recoverable without corrupting the authoritative Note.

## Later / separate decisions

- Semantic/vector search, Laya relevance gating, and Float passage suggestions are optional layers **after** fast lexical search works. They must not gate the manual workflow.
- Whether a user query should retrieve a single matching Segment, a speaker-homogeneous context chunk, or both is a relevance/UI decision to validate.
- Decide FTS tokeniser and support for misspellings, stemming, multilingual transcripts and very short terms from observed research queries.
- Resolve DB-as-projection versus DB-as-canonical-transcript in an ADR before changing persistence ownership.

## Out of scope

Automatic tagging, grouping, reflection, report writing, cloud search, a knowledge graph, or a new Knowledge Artifact store.
