# Hydration prompt test kit — manual runs in Ollama UI

> Status: **active exploration — test assets**. Companion to
> [`2026-07-01-context-hydration-pipeline.md`](./2026-07-01-context-hydration-pipeline.md) §9.1.
> Nothing here is app code. Run the prompts by hand in the Ollama UI, compare against the
> expected-results tables, record what happened in §5.

## Lessons this kit encodes (from the failed "Socrates" run, 2026-07-02)

The previous prompt asked, per phrase, "does this contain an implicit variable — a which-one,
whose, when-exactly?" The model flagged nearly everything, including "nightly sync job" inside
the very sentence that defines it. Diagnosis, carried into this kit:

- **Every noun phrase has a "which one" if you go looking.** "Which reporting database?" is true
  but irrelevant — the sentence is fully understandable without the answer. The question must be
  about whether the *sentence stops making sense*, not whether further facts exist.
- **Yes/no self-containedness is a holistic judgment small models can't hold steady** (already
  observed in the hydration doc §9.1). So: force generation first. The model writes what the
  phrase refers to, *then* labels where that answer came from. A guessed referent is the signal
  for unresolved — and a guess is much harder to disguise than a "false" is to emit.
- **Examples in the prompt bias small models** — the Socrates output parroted the "which X"
  example pattern for every row. These prompts contain no worked examples.
- **Persona framing wastes capacity and tilts the model.** Playing a skeptic biases toward
  claiming ignorance. Plain instructions only.
- **Formatting hygiene:** straight ASCII quotes only (the Socrates output drifted between
  `self_contained` and `self-contained` keys), temperature ≤ 0.2, one chunk per run.

## 1. Pipeline shape

Two prompts, run in sequence per chunk:

- **Prompt 1 — extract.** Mechanical listing of candidate phrases. No judgment. (This step
  already worked in the Socrates run; kept minimal.)
- **Prompt 2 — resolve.** Paste the same chunk plus Prompt 1's phrase list. For each phrase the
  model writes one sentence saying what it refers to, then labels the source of that answer:
  - `text` — the chunk itself explains it → feeds `defines`
  - `general` — the phrase describes itself; everyday work vocabulary suffices → self-contained
  - `missing` — the referent lives in history the reader doesn't have; the sentence would be a
    guess → feeds `unresolved`

Mapping back to the hydration doc's chunk-call output: `text` rows populate `defines`,
`missing` rows populate `unresolved`, `general` rows populate neither.

## 2. Prompt 1 — extract

```
Read this transcript chunk. Each line has this format:

[timestamp] Speaker: spoken words

The timestamp and speaker name are metadata. Use only the spoken words.

List every phrase from the spoken words that points at a specific thing:
names of people, companies, products, or systems; acronyms; phrases built
on "the", "that", "this", or a possessive; references to times or places.
Keep each phrase whole, exactly as written, including its qualifiers.
Do not list filler words, timestamps, or speaker names.

Return a JSON array of strings and nothing else.

Transcript chunk:
"""
<CHUNK>
"""
```

## 3. Prompt 2 — resolve

```
Read this transcript chunk. Each line has this format:

[timestamp] Speaker: spoken words

Below the chunk is a list of phrases taken from it. For each phrase, first
write one short sentence saying what the phrase refers to in this chunk.
Then label where your answer came from, using exactly one of these words:

text - this chunk itself explains what the phrase refers to
general - the phrase describes itself; anyone with everyday work vocabulary
understands the sentence without more information
missing - the answer would be a guess; what it refers to was established
somewhere you cannot see, and without that the sentence is hard to understand

Do not use "missing" just because you lack details about the thing. Use
"missing" only when not knowing what the phrase points at makes the
sentence itself unclear.

Return JSON only, and nothing else:

[
  {"phrase": "...", "refers_to": "...", "source": "text"}
]

Transcript chunk:
"""
<CHUNK>
"""

Phrases:
<PHRASES — one per line, exactly as returned by Prompt 1>
```

## 4. Test chunks and expected results

Run each chunk through both prompts separately. Score Prompt 2 against the tables. The two
failure modes to watch (both seen in the Socrates run):

- **A defined term flagged as missing** — chunk A's "nightly sync job" MUST come back `text`.
- **Over-flagging compositional phrases** — chunk C MUST come back with zero `missing`.

Borderline rows are marked; don't count them as failures either way.

### Chunk A — definition present (verbatim from the Socrates run, for direct comparison)

```
[00:03:12] Sarah: So the nightly sync job — that's the process that pulls the CRM export at 2am, dedupes it against last week's file, and pushes the clean version into the reporting database before anyone's at their desk. It's been running since March.
```

| Phrase | Expected source | Note |
|---|---|---|
| the nightly sync job | text | The sentence defines it. The load-bearing row. |
| the CRM export | general | Compositional; self-describing |
| 2am | general | May not even be extracted; fine either way |
| last week's file | general | The sentence implies it: the previous run's export |
| the reporting database | general | Compositional; self-describing |
| March | general | |

### Chunk B — compressed shorthand (genuine unresolved; pairs with chunk A across chunks)

```
[00:12:04] Dan: Acme pushed back again, so let's park that until after the QBR and just run the usual pipeline on Friday's numbers.
```

| Phrase | Expected source | Note |
|---|---|---|
| Acme | missing | Unintroduced proper noun |
| the QBR | missing | The §9.1 edge case: the acronym is generically definable, but WHOSE/WHICH QBR is history. Accept `general` as a known miss to record, not a pass. |
| the usual pipeline | missing | Semantic partner of chunk A's "nightly sync job" — the cross-chunk match test from §9.1 |
| Friday's numbers | missing | Which numbers is established elsewhere |

### Chunk C — control: nothing needs history (over-flagging detector)

```
[00:08:40] Priya: I think we should add a search box to the settings page so people can find options without scrolling through every section.
```

| Phrase | Expected source | Note |
|---|---|---|
| a search box | general | |
| the settings page | general | Definite article, but the sentence is fully clear |
| every section | general | May not be extracted; fine |

**Pass condition: zero `missing` rows.** Any `missing` here is the over-flagging failure mode.

### Chunk D — written self-note (hardest case per hydration doc §4; no speaker line format)

```
Ask J about the export before Thursday. If the numbers still don't match, fall back to the manual version.
```

For this chunk, drop the "[timestamp] Speaker:" framing lines from both prompts, or just
present it as-is — noting how the prompts degrade on non-transcript input is part of the test.

| Phrase | Expected source | Note |
|---|---|---|
| J | missing | |
| the export | missing | Unlike chunk A's "CRM export", no descriptor: WHICH export is the whole meaning |
| Thursday | general | Borderline: nearest upcoming Thursday is a fair default reading |
| the numbers | missing | |
| the manual version | missing | Manual version of what |

### Chunk E — mixed, with an in-chunk repair (tests that `text` beats `missing` when both apply)

```
[00:21:47] Maya: The blocker is still the handoff doc.
[00:21:52] Tom: Which one?
[00:21:54] Maya: The one for the payments migration — the checklist we started after the outage, listing what the new team needs before we hand the service over.
```

| Phrase | Expected source | Note |
|---|---|---|
| the handoff doc | text | Repaired two lines later — the conversational-repair case transcripts give us for free |
| the payments migration | general | Borderline: compositional enough to parse, but arguably a project name. Record, don't score. |
| the outage | missing | Which outage is pure history |
| the new team | missing | Borderline toward general; record what happens |
| the service | general | Borderline; resolved by "hand the service over" context |

## 5. Results log

| Date | Model | Chunk | Result summary |
|---|---|---|---|
| 2026-07-02 | (Socrates prompt, pre-kit) | A | Failed: flagged defined term + compositional phrases as not self-contained |
| | | | |

## What a pass looks like

Chunk A row 1 comes back `text`, chunk C has zero `missing`, and chunks B/D flag the genuine
shorthand. If that holds across a couple of models/temperatures, the §9.1 load-bearing
assumption survives and the `unresolved`/`defines` recipe can go into a Context Config. If
chunk A row 1 keeps failing, the next variable to isolate is Prompt 2's `text` definition —
before touching anything else, since generation-then-label was this kit's main bet.
