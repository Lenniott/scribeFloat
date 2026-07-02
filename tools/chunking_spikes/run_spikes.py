#!/usr/bin/env python3
"""Chunking spikes: cut a transcript into chunks at conversation boundaries.

Runs four strategies over the same transcript and scores each one against
hand-marked good boundaries (lines containing only `---` in the transcript):

  fixed      cut every N words (the baseline to beat)
  turns      cut only where a speaker's turn ends, once the chunk is big enough
  rules      cut at conversation signals (long pause for THIS conversation,
             closing words); never cut between a question and its answer,
             never cut right before a repair ("which one?")
  rules+sim  rules propose a cut, meaning-similarity confirms it. Similarity
             is word overlap by default, or embeddings via a local Ollama
             server with --embed-model.

Stdlib only, no venv. Run:

  python3 run_spikes.py                          # table + scores
  python3 run_spikes.py --show-chunks            # also print every chunk
  python3 run_spikes.py --transcript my.txt      # your own file
  python3 run_spikes.py --embed-model embeddinggemma:latest
"""
from __future__ import annotations

import argparse
import json
import math
import re
import sys
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from statistics import mean, median, pstdev

LINE_RE = re.compile(r"^\[(\d{2}):(\d{2}):(\d{2})\]\s+([^:]+):\s*(.+)$")
WORD_RE = re.compile(r"[a-z0-9']+")

# Short "I'm listening" noises and go-aheads. They ride inside the other
# person's turn instead of breaking it.
BACKCHANNELS = {
    "yeah", "yep", "yes", "no", "ouch", "right", "okay", "ok", "sure",
    "exactly", "totally", "mm hm", "mm", "hm", "mhm", "uh huh", "huh",
    "wow", "really", "true", "fair", "nice", "go on", "got it",
    "makes sense", "i see", "of course", "oh", "oh no", "oh wow",
}

# Words that, on their own, wrap a topic up ("okay great, thanks").
CLOSER_WORDS = {
    "okay", "ok", "cool", "great", "perfect", "thanks", "thank", "you",
    "alright", "good", "sounds", "done", "deal", "works", "fine", "sweet",
}

# Openers that usually start a question when there's no "?" to lean on.
QUESTION_STARTERS = {
    "what", "why", "how", "when", "where", "which", "who", "whose",
    "can", "could", "do", "does", "did", "is", "are", "was", "were",
    "will", "would", "should", "shall",
}

# Phrases that signal "I didn't follow, fix it" — the fix must stay in the
# same chunk as the thing it fixes.
REPAIR_PREFIXES = (
    "which one", "what do you mean", "sorry", "wait", "hang on",
    "say that again", "you mean", "come again", "the what",
)

STOPWORDS = set(
    """a an the and or but so to of in on for with at from by as is are was
    were be been being it its this that these those there here i you we they
    he she him her them us our your my me his their do does did done not no
    yes if then than up down out over about into after before behind one
    thing things really just very too also still even can could will would
    should may might must have has had having get gets got go goes going
    gone come came comes say said says see saw seen know knew known think
    thought want wanted need needs needed make made let lets new old more
    most much many some all every each other another way ready mm hm mhm uh
    um yeah yep okay ok right sure""".split()
)


@dataclass
class Line:
    idx: int
    ms: int
    speaker: str
    text: str


@dataclass
class Turn:
    speaker: str
    lines: list
    embedded: list = field(default_factory=list)
    last_ms: int = 0

    @property
    def start_ms(self):
        return self.lines[0].ms

    def all_lines(self):
        return sorted(self.lines + self.embedded, key=lambda l: l.idx)

    def last_line(self):
        return self.all_lines()[-1]

    def end_idx(self):
        return self.last_line().idx

    def word_count(self):
        return sum(len(words(l.text)) for l in self.all_lines())

    def text(self):
        return " ".join(l.text for l in self.all_lines())


@dataclass
class Candidate:
    """A possible cut point between two turns."""

    after_idx: int
    gap_ms: int
    legal: bool
    strength: int
    reasons: list
    overlap: float | None = None


def words(text: str) -> list:
    return WORD_RE.findall(text.lower())


def norm(text: str) -> str:
    return " ".join(words(text))


def is_backchannel(text: str) -> bool:
    w = words(text)
    return 0 < len(w) <= 3 and " ".join(w) in BACKCHANNELS


def is_question(text: str) -> bool:
    t = text.rstrip()
    if t.endswith("?"):
        return True
    if t[-1:] in ".!":  # written as a statement — punctuation wins
        return False
    w = words(text)
    return bool(w) and w[0] in QUESTION_STARTERS and len(w) <= 12


def is_repair(text: str) -> bool:
    return norm(text).startswith(REPAIR_PREFIXES)


def is_closer(text: str) -> bool:
    w = words(text)
    return 0 < len(w) <= 4 and all(x in CLOSER_WORDS for x in w)


def parse_transcript(path: Path):
    lines, gold = [], set()
    for raw in path.read_text(encoding="utf-8").splitlines():
        s = raw.strip()
        if not s:
            continue
        if set(s) == {"-"}:
            if lines:
                gold.add(lines[-1].idx)
            continue
        m = LINE_RE.match(s)
        if not m:
            sys.exit(f"unparseable line: {s!r} (expected '[hh:mm:ss] Name: text')")
        h, mn, sec, name, text = m.groups()
        ms = (int(h) * 3600 + int(mn) * 60 + int(sec)) * 1000
        lines.append(Line(len(lines) + 1, ms, name.strip(), text.strip()))
    if not lines:
        sys.exit("transcript is empty")
    return lines, gold


def gap_threshold_ms(lines) -> int:
    """A pause is 'long' relative to THIS conversation's own rhythm."""
    gaps = [b.ms - a.ms for a, b in zip(lines, lines[1:])]
    if not gaps:
        return 8_000
    return int(min(max(3.0 * median(gaps), 6_000), 60_000))


def build_turns(lines, thr):
    turns: list[Turn] = []
    for i, ln in enumerate(lines):
        cur = turns[-1] if turns else None
        gap = ln.ms - cur.last_ms if cur else 0
        if cur and ln.speaker != cur.speaker and is_backchannel(ln.text) and gap < thr:
            nxt = lines[i + 1] if i + 1 < len(lines) else None
            if nxt and nxt.speaker == cur.speaker:
                cur.embedded.append(ln)
                cur.last_ms = max(cur.last_ms, ln.ms)
                continue
        if cur and ln.speaker == cur.speaker and gap < thr:
            cur.lines.append(ln)
            cur.last_ms = ln.ms
        else:
            turns.append(Turn(ln.speaker, [ln], last_ms=ln.ms))
    return turns


def candidates_between(turns, thr):
    out = []
    for i in range(len(turns) - 1):
        prev, nxt = turns[i], turns[i + 1]
        gap = nxt.start_ms - prev.last_ms
        last = prev.last_line()
        reasons, legal, strength = [], True, 0
        if is_question(last.text):
            legal = False
            reasons.append("open question")
        if is_repair(nxt.lines[0].text):
            legal = False
            reasons.append("repair follows")
        if legal:
            if gap >= thr:
                strength += 2
                reasons.append(f"long pause {gap / 1000:.0f}s")
            if is_closer(last.text):
                strength += 2
                reasons.append("closing words")
            if last.text.rstrip()[-1:] in ".!?":
                strength += 1
                reasons.append("finished sentence")
        out.append(Candidate(prev.end_idx(), gap, legal, strength, reasons))
    return out


# ── strategies ──────────────────────────────────────────────────────────────


def fixed_boundaries(lines, budget):
    b, count = set(), 0
    for ln in lines[:-1]:
        count += len(words(ln.text))
        if count >= budget:
            b.add(ln.idx)
            count = 0
    return b


def turn_boundaries(turns, budget):
    b, count = set(), 0
    for t in turns[:-1]:
        count += t.word_count()
        if count >= budget:
            b.add(t.end_idx())
            count = 0
    return b


def rules_boundaries(turns, cands, cap):
    b, count = set(), 0
    for t, c in zip(turns, cands + [None]):
        count += t.word_count()
        if c is None:
            break
        if c.legal and (c.strength >= 2 or count >= cap):
            b.add(c.after_idx)
            count = 0
    return b


def rules_sim_boundaries(turns, cands, cap):
    """Rules propose, similarity confirms: veto a strong cut only when the
    two sides still overlap unusually much for this conversation (mean plus
    one standard deviation over all legal cut points)."""
    legal = [c.overlap for c in cands if c.legal and c.overlap is not None]
    cutoff = (mean(legal) + pstdev(legal)) if len(legal) > 1 else 1.0
    b, count = set(), 0
    for t, c in zip(turns, cands + [None]):
        count += t.word_count()
        if c is None:
            break
        confirmed = c.overlap is not None and c.overlap <= cutoff
        if c.legal and ((c.strength >= 2 and confirmed) or count >= cap):
            b.add(c.after_idx)
            count = 0
    return b


# ── similarity ──────────────────────────────────────────────────────────────


def window_texts(turns, i, k=2):
    prev = " ".join(t.text() for t in turns[max(0, i - k + 1): i + 1])
    nxt = " ".join(t.text() for t in turns[i + 1: i + 1 + k])
    return prev, nxt


def content_words(text):
    return {w for w in words(text) if w not in STOPWORDS}


def jaccard(a, b):
    A, B = content_words(a), content_words(b)
    if not A or not B:
        return 0.0
    return len(A & B) / len(A | B)


def cosine(a, b):
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(x * x for x in b))
    return dot / (na * nb) if na and nb else 0.0


def ollama_embed(texts, model, host):
    req = urllib.request.Request(
        host.rstrip("/") + "/api/embed",
        data=json.dumps({"model": model, "input": texts}).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.loads(r.read())["embeddings"]


def fill_overlaps(turns, cands, embed_model, host):
    pairs = [window_texts(turns, i) for i in range(len(cands))]
    if embed_model:
        try:
            flat = [t for pair in pairs for t in pair]
            vecs = ollama_embed(flat, embed_model, host)
            for i, c in enumerate(cands):
                c.overlap = cosine(vecs[2 * i], vecs[2 * i + 1])
            return f"embeddings ({embed_model})"
        except Exception as e:  # noqa: BLE001 — spike: fall back, keep going
            print(f"[warn] embedding call failed ({e}); using word overlap\n")
    for c, (prev, nxt) in zip(cands, pairs):
        c.overlap = jaccard(prev, nxt)
    return "word overlap (jaccard)"


# ── scoring and report ──────────────────────────────────────────────────────


def score(pred, gold, tol=1):
    pred_s, used, tp = sorted(pred), set(), 0
    for g in sorted(gold):
        best = None
        for p in pred_s:
            if p in used or abs(p - g) > tol:
                continue
            if best is None or abs(p - g) < abs(best - g):
                best = p
        if best is not None:
            used.add(best)
            tp += 1
    prec = tp / len(pred_s) if pred_s else 0.0
    rec = tp / len(gold) if gold else 0.0
    f1 = 2 * prec * rec / (prec + rec) if prec + rec else 0.0
    return prec, rec, f1


def chunks_from(lines, boundaries):
    chunks, cur = [], []
    for ln in lines:
        cur.append(ln)
        if ln.idx in boundaries:
            chunks.append(cur)
            cur = []
    if cur:
        chunks.append(cur)
    return chunks


def print_chunks(name, lines, boundaries):
    print(f"\n### {name}")
    for n, chunk in enumerate(chunks_from(lines, boundaries), 1):
        print(f"  chunk {n}:")
        for ln in chunk:
            print(f"    {ln.speaker}: {ln.text}")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--transcript", type=Path,
                    default=Path(__file__).parent / "transcript.txt")
    ap.add_argument("--budget", type=int, default=60,
                    help="target words per chunk for the size-based strategies")
    ap.add_argument("--embed-model", default=None,
                    help="Ollama embedding model; word overlap if not set")
    ap.add_argument("--ollama-host", default="http://localhost:11434")
    ap.add_argument("--show-chunks", action="store_true")
    args = ap.parse_args()

    lines, gold = parse_transcript(args.transcript)
    thr = gap_threshold_ms(lines)
    turns = build_turns(lines, thr)
    cands = candidates_between(turns, thr)
    sim_name = fill_overlaps(turns, cands, args.embed_model, args.ollama_host)
    cap = args.budget * 3

    total_words = sum(len(words(l.text)) for l in lines)
    print(f"transcript: {len(lines)} lines, {total_words} words, "
          f"{len(turns)} turns")
    print(f"long-pause threshold: {thr / 1000:.1f}s "
          f"(3 x this conversation's median gap)")
    print(f"similarity: {sim_name}")
    if gold:
        print(f"marked good boundaries after lines: {sorted(gold)}")

    print("\ncut-point candidates (between turns):")
    print(f"  {'after':>5}  {'gap':>5}  {'overlap':>7}  signals")
    for c in cands:
        state = " | ".join(c.reasons) if c.reasons else "-"
        block = "" if c.legal else "  [NO CUT ALLOWED]"
        print(f"  {c.after_idx:>5}  {c.gap_ms / 1000:>4.0f}s  "
              f"{c.overlap:>7.2f}  {state}{block}")

    strategies = {
        "fixed": fixed_boundaries(lines, args.budget),
        "turns": turn_boundaries(turns, args.budget),
        "rules": rules_boundaries(turns, cands, cap),
        "rules+sim": rules_sim_boundaries(turns, cands, cap),
    }

    print(f"\n{'strategy':<10} {'chunks':>6} {'cuts after lines':<28}"
          + ("precision  recall  f1" if gold else ""))
    for name, b in strategies.items():
        row = f"{name:<10} {len(b) + 1:>6} {str(sorted(b)):<28}"
        if gold:
            p, r, f1 = score(b, gold)
            row += f"{p:>9.2f} {r:>7.2f} {f1:>5.2f}"
        print(row)

    if args.show_chunks:
        for name, b in strategies.items():
            print_chunks(name, lines, b)


if __name__ == "__main__":
    main()
