# PRD: Research regroup — collect, group, tag and reflect

Status: Draft / exploratory
Product: ScribeFloat
Effort: research-regroup
Date: 2026-09-20

## Product goal
Enable a researcher to find exact transcript evidence, collect several passages, organise them into meaningful groups, tag those groups and write optional reflections that remain linked to original sources. The unit of interpretation is the evidence group, not an individual quote or retrieval chunk.

## Problem
A transcript is easy to store but hard to reuse. Researchers must rediscover quotations, transfer them into another workspace, regroup them and reconstruct their source context. Per-quote mandatory tagging/reflection interrupts reading. During prototyping, the researcher wanted separately tagged passages within one chunk, then preferred collecting and grouping first, tagging and reflecting on groups later.

## Core workflow
Find → Collect → Group → Tag → Reflect.

1. Search transcript and open results with surrounding context.
2. Select any number of distinct exact passages from each result. Keep a tray of ungrouped evidence; do not force tags or reflections during collection.
3. Create named evidence groups; multiselect and move, remove or reorder cards. Several chunks can contribute to one group. A group can remain empty or untitled while working.
4. Tag groups, not necessarily all their individual quotes, with editable vocabulary. Initial research candidates: Workflow, Concern, Requirement, Risk, Environment, Outcome, Concept, Feedback.
5. Optionally type or dictate researcher reflections on a group, referencing the supporting passage cards. Distinguish verbatim quotations from researcher interpretation.

## Behaviour and requirements
- A retrieval chunk is a search/context window, not the persisted unit of evidence.
- A single chunk can provide multiple distinct cards, including overlapping ranges.
- Each card preserves immutable source provenance: Note, Source, transcript line indexes, speaker/time if available; opening the card returns to its place in the original Note.
- Retain evidence and work when switching search results, views or app sessions.
- Support moving cards among groups and leaving them ungrouped. Group deletion must not silently delete evidence or reflections.
- Tags and reflections are optional. No prompt should interrupt collecting the next passage.
- Offer keyboard/explicit Add and Move controls; drag-and-drop can be a refinement.
- Researchers, not AI, decide group meaning and final interpretations.
- Cross-Note grouping is a target; first release may be constrained to one Note if chosen explicitly.

## Suggested surface
Transcript and search alongside a persistent collected-evidence tray / group workspace. Selecting a passage produces a Collect action. The tray allows multi-select → Add to group. Opening a group exposes its title, ordered source cards, tags and reflection. Source text and researcher-authored reflection must remain visually distinguishable.

## Float-assisted follow-on, not manual MVP
Use Float to propose zero or more relevant passage ranges per retrieved chunk and optional tags or grouping suggestions. User can discard, edit or accept suggestions. Float must not silently author a reflection or commit groups. Process chunks independently and progressively; shared prompt prefix caching is a testable optimisation, not a dependency. Return source line indexes and hydrate exact text in the app rather than letting the model recompose quotes. Manual mode works offline without the additional inference model.

Synthetic local tests to date: complete-quote/tag response 2.30 seconds, compact sentence IDs 0.71 seconds, single-chunk retries 0.45 and 0.40 seconds. A five-chunk prompt produced only two result lines. These runs do not establish prompt-cache benefit, multi-chunk completeness or production latency.

## ScribeFloat integration constraints
CONTEXT.md defines Note as the primary object and Transcript as a Source type. Float Layer means a named extraction type with Vocabulary; do not redefine Layer to mean an evidence group. Float and Knowledge are described as future phases, not existing shipped surfaces.

ADR-0015 establishes Whisper Segment as the transcript atom and retrieval chunks as pointers using note_id and segment_indexes. Use existing Note/Source IDs and actual segment indexes, not copied authoritative quotation strings. The experiment's L1–L6 IDs are synthetic. Check the current transcript migration before implementation.

Proposed logical objects (subject to design):
- Evidence card: ID, Note/Source reference and exact indexed range.
- Evidence group: ID, workspace scope, name, ordered evidence references and editable tags.
- Reflection: ID, researcher-authored text, group and explicit evidence references.

Keep all content local under existing privacy commitments; no cloud calls or telemetry.

## Acceptance criteria
- Find a quote in context and collect at least three independently adjustable ranges from one transcript without tagging or reflecting.
- Every saved card reopens the exact source passage and context.
- Create two groups, transfer cards between them, leave a card ungrouped and reorder cards.
- Apply different tags to different groups and leave a group untagged.
- Write, edit or skip reflections; distinguish them from quotations.
- Work survives app restart and works without AI.
- Removing a group does not silently destroy researcher-created material.
- Keyboard-accessible controls exist for collection and grouping.

## Evaluation
Hypothesis: group-first source-linked regroup takes less interaction/time than conventional quote-by-quote workflows without harming evidence fidelity.

Compare conventional manual, group-first manual and (later) Float-assisted tasks on equivalent real transcripts; vary task and order. Measure elapsed time to useful organised evidence, navigation/selection corrections, interactions, source-reference errors, quality/reusability of resulting groups and reflections, and AI correction burden. Do not use click count alone as quality.

## Open decisions
1. One-Note first vs cross-Note research workspace?
2. Can the same evidence card belong to multiple groups, or is membership exclusive?
3. Should initial tags integrate the future Float Layer/Vocabulary or use a smaller research-specific mechanism?
4. Where should groups/reflections persist? Do not assume a new database or override the future Knowledge markdown ADR.
5. What happens to reflection links when evidence moves?
6. Line-based range selection first or character offsets?
7. How do research plans influence search and proposed groupings, and when?

## Out of scope
Automatic research conclusions/personas/reports; forced AI; a prescribed graph taxonomy; GraphQL or embeddings as an architectural prerequisite; a new cloud/account system; replacing the Note/Source domain.

Implementation should follow resolution of the open decisions and validation of the manual interaction.