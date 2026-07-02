# Chunking spikes

Small test scripts for one question: **where should we cut a transcript into
chunks?** A chunk is one piece of a transcript that we store and search on
its own. If we cut in the wrong place — say, between a question and its
answer — the piece makes no sense to anyone reading it later without the
rest. Since a language model reading a chunk has no memory of the rest of
the conversation, wrong cuts are expensive.

The usual approach is to cut every X words. These spikes test a better idea:
conversations already have natural joints, so cut at the joints.

## The four strategies (worst to best)

| Strategy | How it cuts |
|---|---|
| `fixed` | Every 60 words, wherever that lands. The baseline to beat. |
| `turns` | Only where a speaker finishes talking, once the chunk is big enough. A **turn** is one person's speech until the other person takes over. |
| `rules` | At conversation signals. Cut at a **long pause** — long compared to this conversation's own rhythm, not a fixed number. Cut after **closing words** ("okay great, thanks"). Never cut between a question and its answer. Never cut before a **repair** — the "which one?" → "the checklist for..." exchange where someone fixes a misunderstanding; the fix must stay with the thing it fixes. Short "mm-hm" / "yeah" noises don't count as the other person taking over. |
| `rules+sim` | Same as `rules`, but each proposed cut is double-checked: if the text on both sides still talks about the same things, the cut is dropped. "Same things" is measured by word overlap, or by **embeddings** — an embedding turns text into a list of numbers so a computer can measure how close two pieces of text are in meaning. |

## Run it

```bash
python3 run_spikes.py                # score all four strategies
python3 run_spikes.py --show-chunks  # also print every chunk
python3 run_spikes.py --embed-model embeddinggemma:latest   # use Ollama embeddings
python3 run_spikes.py --transcript path/to/yours.txt        # your own transcript
```

No installs needed — plain Python 3.

## Reading the output

The candidate table shows every spot between two turns where a cut *could*
go, with the signals found there. `[NO CUT ALLOWED]` means a rule forbids
cutting there (open question or repair). The score table compares each
strategy's cuts to the hand-marked good cuts:

- **precision** — of the cuts it made, how many were right
- **recall** — of the right cuts, how many it found
- **f1** — the two combined into one number (1.00 is perfect)

Current result on the sample: `fixed` scores 0.00, `rules` and `rules+sim`
score 1.00.

## Testing with your own transcript

Format one line per utterance: `[hh:mm:ss] Name: what they said`.
Put a line containing only `---` after each spot where you think a chunk
should end. Those marks become the answer key the strategies are scored
against. No marks = no scores, but the cuts still print.

## What this feeds

The chunks made here go to two later stages (not in this folder):
each chunk gets an embedding for search, and a separate pipeline checks how
**hydrated** each chunk is — whether it can be understood with no outside
context — and pairs starved chunks with the chunks that explain them. See
`docs/explorations/active/2026-07-02-turn-aware-chunking-and-voice-labeling.md`
(this spike is Layer 3 there) and
`docs/explorations/active/2026-07-01-context-hydration-pipeline.md`.

## Honest limits

- The sample transcript is hand-written and small; the rules were tuned on
  it. The real test is marking up a few genuine recordings and re-running.
- Question and repair spotting lean on punctuation and short phrase lists;
  Whisper's punctuation is imperfect, so expect misses on real transcripts.
- `rules+sim` only shows its value on transcripts where a long pause does
  NOT mean a topic change (speaker thinking mid-story). The sample has no
  such trap yet — add one when testing on real data.
