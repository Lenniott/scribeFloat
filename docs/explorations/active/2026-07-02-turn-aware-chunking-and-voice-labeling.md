# Turn-aware chunking and voice labeling

> Status: **active exploration — current design**. Companion to
> [`2026-07-01-context-hydration-pipeline.md`](./2026-07-01-context-hydration-pipeline.md): that
> doc fixes the chunk→block→pack architecture and leaves `chunk_strategy: turn_taking_aware` as a
> named-but-unspecified recipe. This doc specifies it, and also covers the layer *below* chunking —
> voice attribution and turn assembly — because the crossover we see today happens there, before
> any chunker runs. Nothing here is built.

---

## 1. The problem as observed

Dual-source recordings (mic + speaker capture) show **crossover**: speech from the other party
appears under my label, turns interleave mid-utterance, and paragraph grouping cuts across
question/answer pairs. Any chunking built on top of this inherits the damage — a chunk with the
wrong voice attached is worse than a chunk with no voice at all, because downstream extraction
("what did *I* decide?") will confidently mis-attribute.

So the work splits into two stacked problems:

1. **Voice labeling** — which channel/person does each piece of text belong to (capture-time,
   deterministic, no model calls).
2. **Chunking** — where do meaning-bearing boundaries fall (enrichment-time, logic + embeddings).

---

## 2. What exists today, and why crossover happens

Pipeline today (`scribe.rs` → `model.rs` → `output/render.rs`):

```
mic WAV (continuous) ─┐
                      ├─ two independent Whisper passes (shared session clock —
speaker segments      │  assemble_speaker_pcm places speaker PCM at true offsets)
  assembled to        │
  timeline ───────────┘
        → merge_dual_source: tag "in:"/"out:", sort by start_ms, exact-suffix dedup
        → render_transcript_body: merge same-source segments with gap < 8s into paragraphs
```

Four concrete defects, in decreasing order of visible damage:

1. **Mic bleed passes the filter.** Without headphones, the other party's audio reaches the mic
   and Whisper transcribes it on the mic channel → labeled as me. The suppression in
   `merge_dual_source` (`model.rs`) requires the *exact same text* as a suffix of the previous
   merged segment within 1.5 s of start. Two Whisper passes over acoustically different signals
   (direct capture vs. room-bounced bleed) almost never produce identical text, so the filter
   rarely fires. This is the primary crossover source.
2. **Sort-by-start slices overlapping turns.** Merging is a single sort on `start_ms`. A long mic
   segment that starts just before several short speaker segments is placed wholly before them,
   even though the speech overlapped — reading order no longer matches interaction order, which
   *reads* as mislabeling even when labels are right.
3. **The dedup can also delete legitimate speech.** If both people genuinely say the same short
   thing ("yeah", "okay") close together, the suffix match kills the second one.
4. **Fixed thresholds and inverted documentation.** Paragraph merge uses a fixed 8 s gap
   (`MERGE_GAP_MS` in `render.rs`) regardless of the conversation's own rhythm. And the doc
   comment on `merge_dual_source` said `in:` = speaker — the opposite of what the code and its
   tests do (`in:` = mic). Fixed in the same commit as this doc; noted here because label
   semantics confusion is itself a defect class.

One structural weakness underneath all of this: **the voice label is baked into the segment text**
(`"in: hello"`). Everything downstream (render grouping, word counting, future chunking) has to
re-parse a string prefix, and any text cleanup that touches the prefix silently destroys
attribution.

---

## 3. The conceptual frame: conversation-analysis units → available signals

Transcripts aren't flat text; they have a natural hierarchy. Mapping each level to the signals we
actually have:

| CA concept | Unit | What it maps to here | Signal available |
|---|---|---|---|
| **TCU** (turn constructional unit) | sentence/phrase | one Whisper segment (VAD-bounded) | per-channel segment timestamps + text |
| **Turn** | continuous speech by one person | maximal run of same-channel TCUs not yielded to the other channel | channel identity, overlap timing, gaps |
| **TRP** (transition relevance place) | point where turn-change is relevant | TCU end with syntactic completion + conversation-relative pause + actual speaker change | punctuation, adaptive gap stats, channel switch |
| **Adjacency pair** | question→answer, request→grant | cross-channel turn pair where first turn projects the second | interrogative/imperative cues + the turn that follows |
| **Sequence** (pair + pre/insert/post expansions) | one accomplished action | the chunk — the unit hydration extracts on | pair detection + expansion attachment rules |
| **Activity** | many sequences, one goal | topic segment | embedding cohesion over turn windows |

Two consequences fall out of this frame:

- **Chunk = sequence, not window.** The hydration doc already says a chunk's value is
  self-containedness. A sequence is the smallest unit where an action *completes* — question
  answered, repair resolved, decision closed with an "okay". Cutting inside one always produces a
  compressed, dependent chunk.
- **Overlap-by-characters is replaced by boundaries-at-TRPs.** No fixed-size overlap between
  chunks; a boundary is only legal at a TRP that is also a sequence boundary.

---

## 4. Layer 1 — voice attribution (capture-time, deterministic)

Fixes crossover at the source. All inputs already exist at the `merge_dual_source` call site:
both 16 kHz PCM buffers on the shared session clock, plus both segment lists. No model calls,
no embeddings — this must stay in the fast offline transcription path.

**4.1 Energy-dominance bleed test.** For each mic segment, compute RMS over the same time window
on *both* PCM buffers. If the speaker channel is active in that window and dominates
(`speaker_rms / mic_rms` above a ratio threshold), the mic segment is likely bleed → drop it
(or keep it flagged `Uncertain`, see 4.4). Direct capture beats room bleed on energy essentially
always, so this is a far stronger test than text equality. A small constant lag tolerance
(~50–150 ms, acoustic + buffer delay) applies when windows are compared.

**4.2 Fuzzy overlap arbitration.** Where a mic segment and a speaker segment overlap in time
*and* their texts are similar but not identical (token-set Jaccard or normalized edit distance
above ~0.6 — the realistic bleed case Whisper produces), keep the higher-energy channel's version
and drop the other. This replaces the exact-suffix dedup entirely, and because it requires
*time overlap + similarity + energy*, it no longer eats legitimate twin utterances ("yeah"/"yeah")
that don't time-overlap or that each dominate their own channel.

**4.3 Backchannels are preserved, not deduped.** A short (< ~1.5 s, ≤ 3 words) utterance from a
small closed lexicon ("yeah", "mm-hm", "right", "okay") that falls *inside* the other channel's
ongoing segment is a continuer, not a turn. It must survive filtering (it's the "third position"
evidence that understanding was achieved — hydration cares about exactly this) but must not
*break* the ongoing turn (see Layer 2).

**4.4 Structured attribution, not text prefixes.** `Segment` gains a field instead of a string
prefix:

```rust
enum Voice { Me, Them, Uncertain }   // rendered as "in:" / "out:" only at display time
struct Segment { start_ms, end_ms, text, voice: Voice, voice_confidence: f32 }
```

Rendering, word counting, and the chunker consume the field; the `in:`/`out:` strings become a
render concern. Existing notes with prefix-embedded labels are read via the current prefix
parsing as a fallback — no migration pass, the fallback just never goes away for old jsonl.
`Uncertain` (energy test ambiguous — e.g. both channels active and similar) renders with a marker
rather than a confident wrong label; same principle as the pack empty state: no answer beats a
wrong one.

---

## 5. Layer 2 — turn assembly (replaces sort + fixed-gap paragraphing)

Input: attributed TCUs from Layer 1. Output: **turns** — the unit rendering shows as a paragraph
and chunking composes into sequences.

- **Turn construction.** A turn is a maximal run of same-voice TCUs where each gap is below the
  *conversation-relative* threshold: median inter-TCU gap of this session × k (floor/ceiling
  clamped). Fluent conversation keeps gaps near zero because speakers project TRPs; a fixed 8 s
  constant is wrong in both directions (splits slow thoughtful monologue, merges distinct fast
  exchanges).
- **Embedded backchannels.** An other-voice continuer (4.3) fully inside a turn's span does not
  terminate the turn. It attaches to the turn as an embedded annotation and renders inline, e.g.
  `(them: mm-hm)`, preserving reading order without shredding the paragraph.
- **True overlap = interruption.** An other-voice utterance that overlaps and *continues past*
  the current turn's end is a real turn-taking event: close the current turn at the last completed
  TCU, start the other's turn. Reading order now follows interaction order instead of raw
  `start_ms` sort.
- **TRP marking.** Each turn end is scored as a TRP candidate: syntactic completion (segment ends
  with `.`/`?`/`!` — Whisper punctuation is imperfect but usable), pause above the adaptive
  threshold, and actual voice change. Turn ends that are also strong TRPs are the only legal
  chunk boundaries for Layer 3.

Rendering change: a paragraph = a turn. The `MERGE_GAP_MS` constant and the prefix-string
grouping in `render_transcript_body` go away.

---

## 6. Layer 3 — sequence chunking for hydration (enrichment-time, logic + embeddings)

This is the concrete spec for `chunk_strategy: "turn_taking_aware"` in the Context Config. It
runs in the Float enrichment queue, where embedding calls are already planned
(`embeddinggemma` via the existing inference service) — never in the capture path.

**6.1 Logic pass first (cheap, deterministic):**

- **First-pair-part detection.** Interrogative form (wh-word start, `?` ending, aux-inversion),
  imperative requests, offers/invitations lexicon. A first pair part *projects* a second: the
  boundary after it is illegal until a same-or-other voice turn plausibly completes the pair.
- **Chaining.** Consecutive Q→A pairs with the same asker (interview shape) chain into one
  candidate sequence rather than n tiny chunks — configurable max chain length.
- **Insert-expansions.** A question *by the answerer* between first and second pair part
  ("which Friday?") is absorbed into the pair's sequence, never a boundary.
- **Repair.** Repair initiators ("what?", "sorry?", "you mean…", "no, I meant…") bind the repair
  turn, its trouble-source turn, and the third-position uptake into one sequence. A chunk
  containing a claim without its repair records the *wrong* content as understood.
- **Sequence-closing thirds.** "okay" / "thanks" / "great" / assessments at a TRP followed by an
  above-threshold gap are strong close signals — they *end* the current sequence and belong to
  it (post-expansion), not to the next chunk.

**6.2 Embedding pass as arbiter (where logic is uncertain):**

- **Cohesion gate at candidate boundaries.** For every boundary the logic pass proposes (or can't
  decide), embed the turn window before and after. A boundary stands only if similarity drops
  below threshold — a TextTiling-style depth test over turn embeddings, not sentence embeddings,
  because turns are the units speakers actually exchanged. This is what keeps adjacency pairs
  together even when the logic pass missed the interrogative (Q and A are semantically linked;
  the cohesion gate vetoes the split).
- **Activity segmentation.** Deeper cohesion valleys over a wider window mark activity boundaries
  ("catching up" → "troubleshooting"). Activities aren't chunks; they're metadata on chunks
  (`activity_hint`) so pack assembly can prefer pulling blocks from one activity when a request
  clearly targets it.

**6.3 Chunk output shape.** A chunk keeps its internal turn structure (who said what, in order) —
this feeds directly into the hydration extraction call, where `unresolved`/`defines` benefits from
knowing *whose* shorthand a phrase is. `chunk_refs` remain segment ranges exactly as the hydration
doc specifies; nothing in that doc's storage model changes.

```json
{
  "chunk_id": "chk_...",
  "turns": [ { "voice": "them", "segment_range": [41, 43] }, ... ],
  "boundary_reason": "closing_third+cohesion_drop",
  "pattern_hint": "qa_chain | extended_telling | repair | unclassified",
  "activity_hint": "act_2"
}
```

`pattern_hint` is the pattern-language idea kept deliberately small: only shapes the logic pass
can already detect for free. Extended tellings (one voice holding many turns with only
continuers from the other) are one chunk, not many — the chaining rule generalized.

---

## 7. Build order

| Phase | What | Why first | Acceptance |
|---|---|---|---|
| **P0** | Energy-dominance bleed filter + fuzzy overlap arbitration in `merge_dual_source` (needs PCM passed in) | Fixes the visible crossover with zero new dependencies | Unit tests with synthetic PCM: bleed dropped, twin "yeah" kept, high-energy channel wins overlap |
| **P1** | `Voice` field on `Segment`, prefix parsing demoted to legacy read fallback + render concern | Unblocks everything downstream; removes the string-prefix fragility | Golden render tests unchanged output for old notes; new notes carry structured voice |
| **P2** | Turn assembly + adaptive gap + backchannel embedding; rendering switches to paragraph-per-turn | Reading order = interaction order; paragraphing matches conversation rhythm | Fixture conversations: interruption splits, continuer doesn't, slow monologue stays whole |
| **P3** | Sequence chunker (logic pass, then embedding cohesion gate) as the `turn_taking_aware` strategy in the hydration pipeline | Needs P1/P2 output; needs Float embedding path (story 0052 lineage) | Labeled fixture transcripts: no split inside adjacency pair/repair; closing third ends chunk; extend the hydration test kit with turn-labeled cases |

P0 is small and self-contained. P3 lands inside the hydration pipeline build, not before it.

---

## 8. Open questions

1. **Energy-ratio threshold calibration.** The bleed-dominance ratio needs tuning against real
   headphone vs. speaker-playback sessions. Likely wants the same treatment as the hydration
   prompt: a small labeled fixture set before trusting it (`hydration_test` pattern).
2. **Whisper punctuation reliability for TRP scoring.** If punctuation proves too noisy on the
   fast model tier, TRP scoring degrades to pause + voice-change only — measure before weighting.
3. **`Uncertain` voice rendering.** Marker text vs. omission vs. dual-attribution — a UX question,
   goes through the design skill when P1 reaches rendering.
4. **Does the cohesion gate need turn-count or token-count windows?** Turn-based windows are
   theoretically right but degenerate on monologue-heavy notes (one voice, giant turns) — may need
   token-budget fallback windows there.
5. **Dictate path.** Single-channel, no turns at all — Layers 2–3 no-op gracefully (one voice,
   structural chunking per the hydration doc's written-source rule), but confirm the shared code
   path doesn't force turn machinery onto it.
