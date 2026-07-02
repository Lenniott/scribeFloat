#!/usr/bin/env python3
"""
Hydration prompt test runner — manual §9.1 validation, automated.

Runs the two-prompt hydration pipeline (extract -> resolve) over the fixed
test chunks below, against a local Ollama, and writes one plain-text report
per invocation into results/. Standard library only: no venv, no pip.

Usage:
  python3 run_test.py                          # gemma3:270m, temp 0, 1 run
  python3 run_test.py --temps 0,0.4,0.8        # temperature sweep
  python3 run_test.py --runs 3                 # repeat runs (variance check)
  python3 run_test.py --model qwen3.5:4b       # different model

Prompts are rev 3 (extract) and rev 2 (resolve) from
docs/explorations/active/2026-07-02-hydration-prompt-test-kit.md.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent
RESULTS_DIR = ROOT / "results"

PROMPT_EXTRACT = '''The timestamp and speaker name are metadata. Use only the spoken words.

List the specific things the spoken words mention: people, companies,
products, systems, documents, acronyms, and references to times or places.

Rules for each item:
- Copy it word for word from the spoken words. Keep "the", "that", "this",
  or a possessive if the phrase has one.
- An item is a short noun phrase, usually 1 to 4 words. It must not
  contain a verb and must not be a whole statement.
- A time or place reference is its own separate item.
- Never list two items where one contains the other.
- Do not list filler words, timestamps, or speaker names.

Return a JSON array of strings and nothing else.

Transcript chunk:
"""
{chunk}
"""
'''

PROMPT_RESOLVE = '''You will be given a transcript chunk and a JSON array of phrases taken from it.
The timestamp and speaker name at the start of each chunk line are metadata.
Use only the spoken words.

For each phrase in the array, in order, output one JSON object with exactly
these three keys:

"phrase" - the phrase, copied unchanged from the input array
"refers_to" - one short sentence saying what the phrase refers to in this chunk
"source" - exactly one of these three words:
  text - this chunk itself explains what the phrase refers to
  general - the phrase describes itself; anyone with everyday work vocabulary
  understands the sentence without more information
  missing - your refers_to sentence would be a guess; what the phrase points
  at was established somewhere you cannot see

Do not use "missing" just because you lack details about the thing. Use
"missing" only when not knowing what the phrase points at makes the sentence
itself unclear.

Do not add, remove, split, or reword any phrase. The output array must have
exactly one object for every phrase in the input array.

Return a JSON array only, and nothing else.

Transcript chunk:
"""
{chunk}
"""

Phrases:
{phrases}
'''

# Each check: pattern (regex, matched case-insensitively against extracted
# phrases), expected source, scored (borderline rows are notes, not scores).
CHUNKS = [
    {
        "id": "chunk1_definition_present",
        "label": "Definition present — should be fully self-contained AND define the sync job",
        "text": "[00:03:12] Sarah: So the nightly sync job — that’s the process that pulls the CRM export at 2am, dedupes it against last week’s file, and pushes the clean version into the reporting database before anyone’s at their desk. It’s been running since March.",
        "checks": [
            {"pattern": r"sync job", "expected": "text", "scored": True,
             "note": "load-bearing row: the sentence defines it"},
            {"pattern": r"crm export", "expected": "general", "scored": True,
             "note": "over-flag detector"},
            {"pattern": r"reporting database", "expected": "general", "scored": True,
             "note": "over-flag detector"},
        ],
        "expect_no_missing": True,
        "no_missing_exceptions": [],
    },
    {
        "id": "chunk2_self_contained",
        "label": "Fully self-contained, nothing to define",
        "text": "[00:07:44] Ben: Okay, let’s go with the card-based layout instead of the list view — we all agreed accessibility was the deciding factor there, since the list view failed the contrast check last sprint.",
        "checks": [
            {"pattern": r"card.based layout", "expected": "general", "scored": True, "note": ""},
            {"pattern": r"contrast check", "expected": "general", "scored": True,
             "note": "the original over-flag case"},
            {"pattern": r"last sprint", "expected": "missing", "scored": False,
             "note": "borderline: WHICH sprint is calendar history; recorded, not scored"},
        ],
        "expect_no_missing": True,
        "no_missing_exceptions": [r"last sprint"],
    },
    {
        "id": "chunk3_compressed_reference",
        "label": "Should flag 'the usual pipeline' / 'Acme' — pipeline should later match chunk 1's defines",
        "text": "[00:14:02] Sarah: Yeah just route the new client data through the usual pipeline like we did for Acme, same as always.",
        "checks": [
            {"pattern": r"usual pipeline", "expected": "missing", "scored": True,
             "note": "semantic partner of chunk 1's defined sync job"},
            {"pattern": r"acme", "expected": "missing", "scored": True,
             "note": "unintroduced proper noun"},
            {"pattern": r"client data", "expected": "general", "scored": True,
             "note": "over-flag detector"},
        ],
        "expect_no_missing": False,
        "no_missing_exceptions": [],
    },
    {
        "id": "chunk4_acronym",
        "label": "Should flag the undefined acronym",
        "text": "[00:19:37] Ben: I still need to finish prepping for the QBR on Friday, Sarah said the numbers aren’t ready yet.",
        "checks": [
            {"pattern": r"qbr", "expected": "missing", "scored": True,
             "note": "generically definable acronym, but WHOSE/WHICH QBR is history"},
            {"pattern": r"friday", "expected": "general", "scored": False,
             "note": "borderline: nearest Friday is a fair default reading"},
            {"pattern": r"numbers", "expected": "missing", "scored": False,
             "note": "borderline: which numbers is established elsewhere"},
        ],
        "expect_no_missing": False,
        "no_missing_exceptions": [],
    },
    {
        "id": "chunk5_written_note",
        "label": "Telegraphic written self-note — several unresolved expected",
        "text": "check w/ D re: budget — same issue as last time, need sign-off before Fri",
        "checks": [
            {"pattern": r"\bd\b", "expected": "missing", "scored": True,
             "note": "who is D"},
            {"pattern": r"last time", "expected": "missing", "scored": True,
             "note": "which previous occasion / which issue"},
            {"pattern": r"budget", "expected": "missing", "scored": False,
             "note": "borderline: which budget, but the word self-describes"},
            {"pattern": r"\bfri\b", "expected": "general", "scored": False,
             "note": "borderline, same as Friday above"},
        ],
        "expect_no_missing": False,
        "no_missing_exceptions": [],
    },
]

VALID_SOURCES = {"text", "general", "missing"}


def call_ollama(url: str, model: str, prompt: str, temperature: float, num_predict: int) -> str:
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": temperature,
            "top_p": 0.9,
            "num_predict": num_predict,
        },
    }
    req = urllib.request.Request(
        url.rstrip("/") + "/api/generate",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=300) as resp:
        return json.loads(resp.read().decode("utf-8")).get("response", "")


def extract_json_array(text: str):
    text = re.sub(r"<think(?:ing)?>.*?</think(?:ing)?>", "", text, flags=re.DOTALL | re.IGNORECASE)
    fenced = re.search(r"```(?:json)?\s*(.*?)```", text, flags=re.DOTALL | re.IGNORECASE)
    if fenced:
        text = fenced.group(1).strip()
    start, end = text.find("["), text.rfind("]")
    if start == -1 or end <= start:
        raise ValueError("no JSON array in response")
    candidate = text[start:end + 1]
    try:
        return json.loads(candidate)
    except json.JSONDecodeError:
        return json.loads(re.sub(r",\s*([}\]])", r"\1", candidate))


def normalise_quotes(text: str) -> str:
    return text.replace("’", "'").replace("‘", "'").replace("“", '"').replace("”", '"')


def strip_article(phrase: str) -> str:
    return re.sub(r"^(the|that|this|a|an)\s+", "", normalise_quotes(phrase).strip().lower())


def extraction_notes(chunk_text: str, phrases: list[str]) -> list[str]:
    """Flag paraphrase and clause-length items — the known Prompt 1 failure modes."""
    notes = []
    lowered = normalise_quotes(chunk_text).lower()
    for p in phrases:
        if strip_article(p) not in lowered:
            notes.append(f"  PARAPHRASE  {p!r} does not appear verbatim in the chunk")
        if len(p.split()) > 5:
            notes.append(f"  CLAUSE?     {p!r} is longer than 5 words")
    for i, a in enumerate(phrases):
        for b in phrases[i + 1:]:
            la, lb = a.lower(), b.lower()
            if la != lb and (la in lb or lb in la):
                notes.append(f"  OVERLAP     {a!r} / {b!r}")
    return notes


def score_chunk(chunk: dict, rows: list[dict]) -> tuple[list[str], dict]:
    lines = []
    tally = {"pass": 0, "mismatch": 0, "not_extracted": 0}
    by_phrase = [(str(r.get("phrase", "")), str(r.get("source", "")).strip().lower()) for r in rows]

    for check in chunk["checks"]:
        rx = re.compile(check["pattern"], re.IGNORECASE)
        matched = [(p, s) for p, s in by_phrase if rx.search(p)]
        tag = "SCORED" if check["scored"] else "NOTE  "
        if not matched:
            lines.append(f"  [{tag}] NOT_EXTRACTED  /{check['pattern']}/ — no phrase matched ({check['note']})")
            if check["scored"]:
                tally["not_extracted"] += 1
            continue
        for phrase, source in matched:
            ok = source == check["expected"]
            verdict = "PASS" if ok else "MISMATCH"
            lines.append(
                f"  [{tag}] {verdict:13} {phrase!r} -> {source or '(no source key)'} "
                f"(expected {check['expected']}; {check['note']})"
            )
            if check["scored"]:
                tally["pass" if ok else "mismatch"] += 1

    if chunk["expect_no_missing"]:
        exceptions = [re.compile(p, re.IGNORECASE) for p in chunk["no_missing_exceptions"]]
        stray = [
            p for p, s in by_phrase
            if s == "missing" and not any(rx.search(p) for rx in exceptions)
        ]
        if stray:
            lines.append(f"  [SCORED] OVER-FLAG: expected zero 'missing', got: {stray}")
            tally["mismatch"] += len(stray)
        else:
            lines.append("  [SCORED] PASS          no over-flagging (zero unexpected 'missing')")
            tally["pass"] += 1

    bad_sources = [(p, s) for p, s in by_phrase if s not in VALID_SOURCES]
    if bad_sources:
        lines.append(f"  [SCORED] BAD SOURCE VALUES: {bad_sources}")
        tally["mismatch"] += len(bad_sources)
    return lines, tally


def run_once(out: list[str], args, temperature: float, run_no: int, totals: dict) -> None:
    for chunk in CHUNKS:
        header = f"--- temp={temperature} run={run_no} {chunk['id']} ---"
        out.append(header)
        out.append(f"({chunk['label']})")
        print(header)

        try:
            raw1 = call_ollama(args.url, args.model, PROMPT_EXTRACT.format(chunk=chunk["text"]),
                               temperature, args.num_predict)
        except urllib.error.URLError as exc:
            sys.exit(f"Could not reach Ollama at {args.url}: {exc}\nIs `ollama serve` (or the app) running?")
        out.append("\n[prompt 1 raw response]")
        out.append(raw1.strip() or "(empty)")
        try:
            phrases = [str(p) for p in extract_json_array(raw1) if str(p).strip()]
        except (ValueError, json.JSONDecodeError) as exc:
            out.append(f"\n!! prompt 1 parse failure: {exc} — skipping resolve step\n")
            totals["parse_failures"] += 1
            continue
        out.append(f"\n[parsed phrases] {json.dumps(phrases, ensure_ascii=False)}")
        notes = extraction_notes(chunk["text"], phrases)
        if notes:
            out.append("[extraction quality flags]")
            out.extend(notes)

        try:
            raw2 = call_ollama(
                args.url, args.model,
                PROMPT_RESOLVE.format(chunk=chunk["text"],
                                      phrases=json.dumps(phrases, ensure_ascii=False)),
                temperature, args.num_predict,
            )
        except urllib.error.URLError as exc:
            sys.exit(f"Could not reach Ollama at {args.url}: {exc}")
        out.append("\n[prompt 2 raw response]")
        out.append(raw2.strip() or "(empty)")
        try:
            rows = [r for r in extract_json_array(raw2) if isinstance(r, dict)]
        except (ValueError, json.JSONDecodeError) as exc:
            out.append(f"\n!! prompt 2 parse failure: {exc}\n")
            totals["parse_failures"] += 1
            continue
        if len(rows) != len(phrases):
            out.append(f"\n!! contract violation: {len(phrases)} phrases in, {len(rows)} objects out")
            totals["contract_violations"] += 1

        out.append("\n[scoring]")
        lines, tally = score_chunk(chunk, rows)
        out.extend(lines)
        for k, v in tally.items():
            totals[k] += v
        out.append("")


def main() -> None:
    parser = argparse.ArgumentParser(description="Hydration prompt pipeline test runner")
    parser.add_argument("--model", default="gemma3:270m")
    parser.add_argument("--url", default="http://localhost:11434")
    parser.add_argument("--temps", default="0",
                        help="comma-separated temperatures, e.g. 0,0.4,0.8")
    parser.add_argument("--runs", type=int, default=1,
                        help="repeat runs per temperature (variance check)")
    parser.add_argument("--num-predict", type=int, default=2048)
    args = parser.parse_args()
    temps = [float(t) for t in args.temps.split(",") if t.strip()]

    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    safe_model = re.sub(r"[^a-zA-Z0-9._-]+", "_", args.model)
    out_path = RESULTS_DIR / f"hydration_{safe_model}_{stamp}.txt"

    totals = {"pass": 0, "mismatch": 0, "not_extracted": 0,
              "parse_failures": 0, "contract_violations": 0}
    out: list[str] = [
        "hydration prompt pipeline test",
        f"date: {datetime.now().isoformat(timespec='seconds')}",
        f"model: {args.model} | url: {args.url}",
        f"temps: {temps} | runs per temp: {args.runs} | num_predict: {args.num_predict}",
        "prompts: extract rev 3, resolve rev 2 (see docs/explorations/active/2026-07-02-hydration-prompt-test-kit.md)",
        "",
    ]

    for temperature in temps:
        for run_no in range(1, args.runs + 1):
            run_once(out, args, temperature, run_no, totals)

    out.append("=== summary ===")
    for key, value in totals.items():
        out.append(f"{key}: {value}")

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(out) + "\n", encoding="utf-8")
    print("\n".join(out[-7:]))
    print(f"\nwrote {out_path}")


if __name__ == "__main__":
    main()
