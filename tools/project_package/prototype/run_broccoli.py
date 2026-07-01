#!/usr/bin/env python3
"""
Broccoli memory prototype.

Purpose:
- Drop transcripts into transcripts/
- Run this file
- Build a small local memory layer across all transcripts
- Build context packs from pack_requests/*.json using those memories

This is a product-learning prototype, not app architecture.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import shutil
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

import requests

ROOT = Path(__file__).resolve().parent
TRANSCRIPTS_DIR = ROOT / "transcripts"
OUTPUTS_DIR = ROOT / "outputs"
INDEX_DIR = ROOT / "index"
PACK_REQUESTS_DIR = ROOT / "pack_requests"
CONTEXT_PACKS_DIR = ROOT / "context_packs"
CONFIG_PATH = ROOT / "config.json"

CATEGORIES = [
    "Situation",
    "Problem",
    "Intent",
    "Option",
    "Decision",
    "Evidence",
    "Open Thread",
]

REQUEST_STOPWORDS = {
    "about",
    "across",
    "brief",
    "build",
    "context",
    "could",
    "create",
    "defining",
    "describe",
    "exact",
    "focu",
    "focus",
    "from",
    "help",
    "idea",
    "ideas",
    "inside",
    "language",
    "later",
    "light",
    "like",
    "make",
    "memories",
    "memory",
    "might",
    "need",
    "open",
    "pack",
    "point",
    "preserving",
    "problem",
    "question",
    "questions",
    "request",
    "retrieves",
    "risk",
    "risks",
    "role",
    "should",
    "solve",
    "solves",
    "source",
    "speaks",
    "that",
    "these",
    "this",
    "those",
    "them",
    "transcript",
    "transcripts",
    "what",
    "when",
    "whether",
    "while",
    "where",
    "with",
    "worth",
}

OPPORTUNITY_LABELS = [
    {
        "label": "Risk assessment and report production",
        "needles": ["risk assessment", "report", "report structure", "questionnaire"],
        "angle": "Light automation could help turn collected client facts into a more consistent risk assessment/report workflow, while keeping advice and review human-owned.",
    },
    {
        "label": "Fact-finding forms and client intake",
        "needles": ["fact-finding", "questionnaire", "form", "review answers", "submit"],
        "angle": "Light automation could reduce setup friction, guide completion, and route answers into the right downstream workflow.",
    },
    {
        "label": "Client data, folders, and document routing",
        "needles": ["google drive", "folder", "client data", "save", "folder id"],
        "angle": "Light automation could standardise where client material lands and reduce manual folder/configuration steps.",
    },
    {
        "label": "Website, SEO, and marketing content workflow",
        "needles": ["seo", "blog", "keyword", "website", "marketing", "organic traffic"],
        "angle": "Light automation could support keyword/question discovery, draft/update loops, and links back to relevant services.",
    },
    {
        "label": "Tax return work feeding planning advice",
        "needles": ["tax return", "tax returns", "planning", "advice", "errors", "inquiries"],
        "angle": "Light automation could help spot planning opportunities and recurring issues inside routine tax-return work.",
    },
]

PACK_SIGNAL_LEXICON = {
    "workflow_discovery": [
        "workflow",
        "process",
        "handoff",
        "manual",
        "repeat",
        "review",
        "submit",
        "follow up",
        "decision",
        "problem",
        "open question",
    ],
    "light_automation": [
        "automation",
        "automated",
        "ai",
        "assist",
        "reduce friction",
        "routing",
        "template",
        "generate",
        "extract",
        "summarise",
    ],
    "client_service": [
        "client",
        "advice",
        "service",
        "inquiry",
        "planning",
        "data",
        "paper",
        "email",
    ],
    "tax": ["tax", "tax return", "tax returns", "hmrc", "return", "allowance"],
    "planning": ["planning", "advice", "financial planning", "life insurance", "property"],
    "intake": ["intake", "questionnaire", "questions", "answers", "client data"],
    "forms": ["form", "forms", "fact-finding", "review answers", "submit"],
    "reports": ["report", "risk assessment", "report structure", "template"],
    "documents": ["document", "folder", "google drive", "save", "file", "inbox"],
    "marketing": ["marketing", "website", "seo", "blog", "keyword", "content", "page"],
}


@dataclass
class Unit:
    unit_id: int
    text: str


def now() -> str:
    return datetime.now().isoformat(timespec="seconds")


def slugify(name: str) -> str:
    stem = Path(name).stem.lower()
    return re.sub(r"[^a-z0-9]+", "-", stem).strip("-") or "note"


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def short_hash(text: str, n: int = 12) -> str:
    return sha256_text(text)[:n]


def read_json(path: Path, default: Any = None) -> Any:
    if not path.exists():
        return default
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")


def append_jsonl(path: Path, row: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")


def load_config() -> dict[str, Any]:
    if not CONFIG_PATH.exists():
        raise SystemExit(f"Missing {CONFIG_PATH}")
    config = read_json(CONFIG_PATH, {})
    config.setdefault("categories", CATEGORIES)
    config.setdefault("temperature", 0.1)
    config.setdefault("num_predict", 4096)
    config.setdefault("think", False)
    config.setdefault("batch_size_units", 120)
    config.setdefault("overlap_units", 12)
    config.setdefault("force_rerun_batches", False)
    config.setdefault("force_rerun_memory", False)
    config.setdefault("force_rerun_embeddings", False)
    config.setdefault("embed_memories", True)
    return config


def ollama_models(config: dict[str, Any]) -> set[str]:
    response = requests.get(config["ollama_url"].rstrip("/") + "/api/tags", timeout=10)
    response.raise_for_status()
    return {m.get("name", "") for m in response.json().get("models", [])}


def check_ollama(config: dict[str, Any]) -> None:
    try:
        models = ollama_models(config)
    except Exception as exc:
        raise SystemExit(f"Could not reach Ollama at {config['ollama_url']}: {exc}") from exc
    if config["model"] not in models:
        raise SystemExit(f"Missing Ollama model `{config['model']}`. Available: {', '.join(sorted(models))}")
    if config.get("embed_memories") and config.get("embedding_model") not in models:
        raise SystemExit(
            f"Missing embedding model `{config.get('embedding_model')}`. Available: {', '.join(sorted(models))}"
        )


def generate(config: dict[str, Any], prompt: str) -> str:
    payload = {
        "model": config["model"],
        "prompt": prompt,
        "stream": False,
        "think": bool(config.get("think", False)),
        "options": {
            "temperature": config.get("temperature", 0.1),
            "top_p": config.get("top_p", 0.9),
            "num_predict": config.get("num_predict", 4096),
        },
    }
    response = requests.post(config["ollama_url"].rstrip("/") + "/api/generate", json=payload, timeout=300)
    response.raise_for_status()
    return response.json().get("response", "")


def embed(config: dict[str, Any], text: str) -> list[float]:
    payload = {"model": config["embedding_model"], "prompt": text}
    response = requests.post(config["ollama_url"].rstrip("/") + "/api/embeddings", json=payload, timeout=300)
    response.raise_for_status()
    data = response.json()
    vector = data.get("embedding")
    if not isinstance(vector, list):
        raise RuntimeError(f"No embedding returned: {data}")
    return vector


def strip_thinking(text: str) -> str:
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL | re.IGNORECASE)
    text = re.sub(r"<thinking>.*?</thinking>", "", text, flags=re.DOTALL | re.IGNORECASE)
    return text.strip()


def extract_json_array(text: str) -> list[Any]:
    text = strip_thinking(text)
    fenced = re.search(r"```(?:json)?\s*(.*?)```", text, flags=re.DOTALL | re.IGNORECASE)
    if fenced:
        text = fenced.group(1).strip()
    start = text.find("[")
    end = text.rfind("]")
    if start == -1 or end == -1 or end <= start:
        raise ValueError("No JSON array found")
    candidate = text[start : end + 1]
    try:
        return json.loads(candidate)
    except json.JSONDecodeError:
        repaired = re.sub(r",\s*([}\]])", r"\1", candidate)
        return json.loads(repaired)


def extract_json_object(text: str) -> dict[str, Any]:
    text = strip_thinking(text)
    fenced = re.search(r"```(?:json)?\s*(.*?)```", text, flags=re.DOTALL | re.IGNORECASE)
    if fenced:
        text = fenced.group(1).strip()
    start = text.find("{")
    end = text.rfind("}")
    if start == -1 or end == -1 or end <= start:
        raise ValueError("No JSON object found")
    candidate = text[start : end + 1]
    return json.loads(re.sub(r",\s*([}\]])", r"\1", candidate))


def unitise(text: str) -> list[Unit]:
    units = []
    pattern = re.compile(r"[^.!?\n]+(?:[.!?]+|\n+|$)", re.MULTILINE)
    for idx, match in enumerate(pattern.finditer(text), start=1):
        cleaned = re.sub(r"\s+", " ", match.group(0)).strip()
        if cleaned:
            units.append(Unit(idx, cleaned))
    return units


def make_batches(units: list[Unit], batch_size: int, overlap: int) -> list[list[Unit]]:
    batches = []
    start = 0
    while start < len(units):
        end = min(start + batch_size, len(units))
        batches.append(units[start:end])
        if end == len(units):
            break
        start = max(end - overlap, start + 1)
    return batches


def extract_text(units_by_id: dict[int, Unit], start: int, end: int) -> str:
    return " ".join(units_by_id[i].text for i in range(start, end + 1) if i in units_by_id).strip()


def excerpt(text: str, limit: int = 300) -> str:
    text = re.sub(r"\s+", " ", text).strip()
    if len(text) <= limit:
        return text
    return text[: limit - 3].rstrip() + "..."


def build_batch_prompt(config: dict[str, Any], note_id: str, batch_num: int, units: list[Unit]) -> str:
    unit_text = "\n".join(f"[{u.unit_id}] {u.text}" for u in units)
    categories = ", ".join(config["categories"])
    return f"""You are analysing one batch from a messy conversation transcript.

Goal:
Find candidate memory occurrences: useful moments that may belong to a recurring conversation thread.

A thread is a line of inquiry, tension, workstream, concern, unresolved decision, or theme that may recur across this transcript or across other transcripts.

The seven aspect categories are: {categories}

Return JSON only.
Do not include reasoning.
Do not include markdown.
Do not include code fences.
Do not include <think> blocks.
Your response must begin with [ and end with ].

Required JSON shape:
[
  {{
    "provisional_thread_summary": "Short neutral description of the possible thread, not a title.",
    "line_of_inquiry": "The question or tension this occurrence contributes to.",
    "unit_start": 10,
    "unit_end": 18,
    "aspect": "Problem",
    "summary": "Neutral summary of what this source range says.",
    "confidence": "medium"
  }}
]

Rules:
- Use exact unit IDs from this batch.
- unit_start and unit_end must be consecutive.
- aspect must be one of the seven categories.
- Select only useful source ranges, not filler.
- Keep ranges tight enough to represent one meaningful occurrence.
- Do not invent task-specific labels.
- Do not create polished titles.

Transcript batch {batch_num} from note {note_id}:

{unit_text}
"""


def clean_candidate(item: dict[str, Any], batch_units: list[Unit], config: dict[str, Any], note_id: str, batch_num: int, units_by_id: dict[int, Unit]) -> dict[str, Any] | None:
    unit_ids = {u.unit_id for u in batch_units}
    try:
        start = int(item.get("unit_start"))
        end = int(item.get("unit_end"))
    except Exception:
        return None
    if start > end:
        start, end = end, start
    if start not in unit_ids or end not in unit_ids:
        return None
    aspect = item.get("aspect")
    if aspect not in config["categories"]:
        return None
    confidence = item.get("confidence") or "medium"
    if confidence not in {"low", "medium", "high"}:
        confidence = "medium"
    text = extract_text(units_by_id, start, end)
    row = {
        "candidate_id": "cand_" + short_hash(f"{note_id}:{start}:{end}:{aspect}:{text}"),
        "note_id": note_id,
        "batch": batch_num,
        "unit_start": start,
        "unit_end": end,
        "aspect": aspect,
        "provisional_thread_summary": str(item.get("provisional_thread_summary") or "").strip()[:500],
        "line_of_inquiry": str(item.get("line_of_inquiry") or "").strip()[:600],
        "summary": str(item.get("summary") or "").strip()[:500],
        "confidence": confidence,
        "text": text,
    }
    return row


def infer_aspect(text: str) -> str:
    lowered = text.lower()
    checks = [
        ("Decision", ["decide", "decided", "agreed", "commit", "committed", "will do", "going to"]),
        ("Problem", ["problem", "risk", "issue", "blocker", "hard", "difficult", "concern", "worry"]),
        ("Intent", ["need", "want", "goal", "trying to", "aim", "purpose", "so that"]),
        ("Option", ["could", "might", "option", "alternative", "tradeoff", "instead"]),
        ("Evidence", ["because", "example", "data", "evidence", "shows", "learned", "found"]),
        ("Open Thread", ["question", "unclear", "not sure", "open", "follow up", "figure out"]),
    ]
    for aspect, needles in checks:
        if any(needle in lowered for needle in needles):
            return aspect
    return "Situation"


def fallback_candidates(note_id: str, batch_num: int, batch_units: list[Unit], units_by_id: dict[int, Unit]) -> list[dict[str, Any]]:
    signal_words = re.compile(
        r"\b(need|want|goal|problem|risk|issue|because|could|might|decid|agreed|question|unclear|important|role|work|context|memory|thread|test|learn)\b",
        re.IGNORECASE,
    )
    rows = []
    for unit in batch_units:
        if not signal_words.search(unit.text):
            continue
        start = unit.unit_id
        end = min(unit.unit_id + 1, batch_units[-1].unit_id)
        text = extract_text(units_by_id, start, end)
        if len(text) < 40:
            continue
        aspect = infer_aspect(text)
        rows.append({
            "candidate_id": "cand_" + short_hash(f"fallback:{note_id}:{start}:{end}:{aspect}:{text}"),
            "note_id": note_id,
            "batch": batch_num,
            "unit_start": start,
            "unit_end": end,
            "aspect": aspect,
            "provisional_thread_summary": "Source-linked conversation memory surfaced by fallback extraction.",
            "line_of_inquiry": "What recurring situation, concern, intention, option, decision, evidence, or open thread does this moment contribute to?",
            "summary": excerpt(text, 240),
            "confidence": "low",
            "text": text,
        })
        if len(rows) >= 12:
            break
    return rows


def process_batch(config: dict[str, Any], note_id: str, batch_num: int, batch_units: list[Unit], run_dir: Path, units_by_id: dict[int, Unit]) -> tuple[list[dict[str, Any]], dict[str, Any] | None]:
    prompt_path = run_dir / "debug" / "prompts" / f"batch_{batch_num:03d}.md"
    response_path = run_dir / "debug" / "model_responses" / f"batch_{batch_num:03d}.txt"
    result_path = run_dir / "checkpoints" / "batch_candidates" / f"batch_{batch_num:03d}.json"
    if result_path.exists() and not config.get("force_rerun_batches"):
        rows = read_json(result_path, [])
        if rows:
            return rows, None
        rows = fallback_candidates(note_id, batch_num, batch_units, units_by_id)
        if rows:
            write_json(result_path, rows)
            print(f"  warning: batch_{batch_num:03d} cached no observations; used fallback extraction")
        return rows, None
    prompt = build_batch_prompt(config, note_id, batch_num, batch_units)
    prompt_path.parent.mkdir(parents=True, exist_ok=True)
    prompt_path.write_text(prompt, encoding="utf-8")
    print(f"  batch_{batch_num:03d}: candidate memory occurrences")
    try:
        raw = generate(config, prompt)
        response_path.parent.mkdir(parents=True, exist_ok=True)
        response_path.write_text(raw, encoding="utf-8")
        parsed = extract_json_array(raw)
        rows = []
        for item in parsed:
            if isinstance(item, dict):
                cleaned = clean_candidate(item, batch_units, config, note_id, batch_num, units_by_id)
                if cleaned:
                    rows.append(cleaned)
        if not rows:
            rows = fallback_candidates(note_id, batch_num, batch_units, units_by_id)
            if rows:
                print(f"  warning: batch_{batch_num:03d} returned no usable observations; used fallback extraction")
        rows = list({row["candidate_id"]: row for row in rows}.values())
        write_json(result_path, rows)
        return rows, None
    except Exception as exc:
        rows = fallback_candidates(note_id, batch_num, batch_units, units_by_id)
        if rows:
            write_json(result_path, rows)
            print(f"  warning: batch_{batch_num:03d} failed; used fallback extraction")
            return rows, {
                "batch": batch_num,
                "error": str(exc),
                "fallback_candidate_count": len(rows),
                "prompt_path": str(prompt_path.relative_to(ROOT)),
                "response_path": str(response_path.relative_to(ROOT)) if response_path.exists() else None,
                "time": now(),
            }
        error = {
            "batch": batch_num,
            "error": str(exc),
            "prompt_path": str(prompt_path.relative_to(ROOT)),
            "response_path": str(response_path.relative_to(ROOT)) if response_path.exists() else None,
            "time": now(),
        }
        print(f"  warning: batch_{batch_num:03d} failed: {exc}")
        return [], error


def build_memory_prompt(config: dict[str, Any], note_id: str, candidates: list[dict[str, Any]]) -> str:
    rows = []
    for cand in candidates:
        rows.append(
            f"- {cand['candidate_id']} | units {cand['unit_start']}-{cand['unit_end']} | {cand['aspect']} | "
            f"possible thread: {cand['provisional_thread_summary']} | inquiry: {cand['line_of_inquiry']} | "
            f"summary: {cand['summary']} | text: {excerpt(cand['text'], 220)}"
        )
    return f"""You are building a compact product-facing thread memory for one transcript.

Goal:
Merge candidate occurrences into durable memory threads.

Important:
- Do not create thread titles.
- Threads should have IDs, summaries, line_of_inquiry, status, confidence, and candidate_ids.
- A thread can be discontinuous: it can appear, disappear, and return later.
- Keep 3 to 8 useful threads if supported.
- Do not force every candidate into a thread. Weak candidates can be left out.

Return JSON only.
Do not include reasoning.
Do not include markdown.
Do not include code fences.
Do not include <think> blocks.
Your response must begin with [ and end with ].

Required JSON shape:
[
  {{
    "thread_id": "thread_001",
    "line_of_inquiry": "What question, tension, or workstream does this thread track?",
    "status": "open",
    "candidate_ids": ["cand_abc", "cand_def"],
    "summary": "Source-grounded summary of this recurring thread.",
    "why_it_matters": "Why this is useful product memory. Keep this clearly inferential.",
    "confidence": "medium"
  }}
]

Allowed status values: open, active, resolved, mixed.

Candidate observations from note {note_id}:

{chr(10).join(rows)}
"""


def clean_thread(item: dict[str, Any], idx: int, valid_ids: set[str]) -> dict[str, Any] | None:
    candidate_ids = item.get("candidate_ids") or []
    if not isinstance(candidate_ids, list):
        candidate_ids = []
    candidate_ids = [cid for cid in candidate_ids if cid in valid_ids]
    if not candidate_ids:
        return None
    status = item.get("status") or "active"
    if status not in {"open", "active", "resolved", "mixed"}:
        status = "active"
    confidence = item.get("confidence") or "medium"
    if confidence not in {"low", "medium", "high"}:
        confidence = "medium"
    thread_id = str(item.get("thread_id") or f"thread_{idx:03d}")
    thread_id = re.sub(r"[^a-zA-Z0-9_-]+", "_", thread_id).strip("_").lower()
    if not thread_id.startswith("thread_"):
        thread_id = f"thread_{idx:03d}"
    return {
        "thread_id": thread_id,
        "line_of_inquiry": str(item.get("line_of_inquiry") or "").strip()[:700],
        "status": status,
        "candidate_ids": candidate_ids,
        "summary": str(item.get("summary") or "").strip()[:900],
        "why_it_matters": str(item.get("why_it_matters") or "").strip()[:900],
        "confidence": confidence,
    }


def fallback_threads(candidates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not candidates:
        return []
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for cand in candidates:
        grouped[cand.get("aspect", "Situation")].append(cand)
    priority = ["Problem", "Intent", "Decision", "Option", "Open Thread", "Evidence", "Situation"]
    threads = []
    for aspect in priority:
        group = grouped.get(aspect, [])
        if not group:
            continue
        sample = group[:5]
        threads.append({
            "thread_id": f"thread_{len(threads) + 1:03d}",
            "line_of_inquiry": f"What does the transcript repeatedly surface as {aspect.lower()} memory?",
            "status": "active" if aspect != "Decision" else "mixed",
            "candidate_ids": [cand["candidate_id"] for cand in sample],
            "summary": " ".join(excerpt(cand["summary"], 180) for cand in sample[:3]),
            "why_it_matters": "Fallback thread created from source-linked observations because the model did not return a usable synthesis.",
            "confidence": "low",
        })
        if len(threads) >= 6:
            break
    return threads


def synthesize_memory(config: dict[str, Any], note_id: str, candidates: list[dict[str, Any]], run_dir: Path) -> tuple[list[dict[str, Any]], dict[str, Any] | None]:
    result_path = run_dir / "checkpoints" / "thread_memory_raw.json"
    prompt_path = run_dir / "debug" / "prompts" / "thread_memory.md"
    response_path = run_dir / "debug" / "model_responses" / "thread_memory.txt"
    if result_path.exists() and not config.get("force_rerun_memory"):
        threads = read_json(result_path, [])
        if threads or not candidates:
            return threads, None
        threads = fallback_threads(candidates)
        if threads:
            write_json(result_path, threads)
            print("  warning: cached memory synthesis had no threads; used fallback synthesis")
        return threads, None
    if not candidates:
        write_json(result_path, [])
        return [], {"error": "No candidate observations produced"}
    prompt = build_memory_prompt(config, note_id, candidates)
    prompt_path.parent.mkdir(parents=True, exist_ok=True)
    prompt_path.write_text(prompt, encoding="utf-8")
    print("  synthesising transcript memory")
    try:
        raw = generate(config, prompt)
        response_path.parent.mkdir(parents=True, exist_ok=True)
        response_path.write_text(raw, encoding="utf-8")
        parsed = extract_json_array(raw)
        valid_ids = {cand["candidate_id"] for cand in candidates}
        threads = []
        for idx, item in enumerate(parsed, start=1):
            if isinstance(item, dict):
                thread = clean_thread(item, idx, valid_ids)
                if thread:
                    threads.append(thread)
        if not threads:
            threads = fallback_threads(candidates)
            if threads:
                print("  warning: memory synthesis returned no usable threads; used fallback synthesis")
        write_json(result_path, threads)
        return threads, None
    except Exception as exc:
        threads = fallback_threads(candidates)
        err = {
            "error": str(exc),
            "fallback_thread_count": len(threads),
            "time": now(),
            "prompt_path": str(prompt_path.relative_to(ROOT)),
            "response_path": str(response_path.relative_to(ROOT)) if response_path.exists() else None,
        }
        if threads:
            write_json(result_path, threads)
            print("  warning: memory synthesis failed; used fallback synthesis")
            return threads, err
        print(f"  warning: memory synthesis failed: {exc}")
        return [], err


def build_memory_object(note_id: str, source_path: Path, source_hash: str, threads: list[dict[str, Any]], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    cands = {cand["candidate_id"]: cand for cand in candidates}
    assigned = set()
    memory_threads = []
    for thread in threads:
        occurrences = []
        for cid in thread.get("candidate_ids", []):
            cand = cands.get(cid)
            if not cand:
                continue
            assigned.add(cid)
            occurrences.append({
                "occurrence_id": "occ_" + short_hash(f"{thread['thread_id']}:{cid}"),
                "candidate_id": cid,
                "note_id": note_id,
                "unit_start": cand["unit_start"],
                "unit_end": cand["unit_end"],
                "category": cand["aspect"],
                "summary": cand["summary"],
                "confidence": cand["confidence"],
                "source_excerpt": excerpt(cand["text"], 450),
                "source_text": cand["text"],
            })
        occurrences.sort(key=lambda x: (x["unit_start"], x["unit_end"]))
        gaps = [occurrences[i]["unit_start"] - occurrences[i - 1]["unit_end"] for i in range(1, len(occurrences))]
        memory_id = f"mem_{note_id}_{thread['thread_id']}"
        memory_threads.append({
            "memory_id": memory_id,
            "note_id": note_id,
            "thread_id": thread["thread_id"],
            "line_of_inquiry": thread.get("line_of_inquiry", ""),
            "status": thread["status"],
            "summary": thread.get("summary", ""),
            "why_it_matters": thread.get("why_it_matters", ""),
            "confidence": thread["confidence"],
            "occurrence_count": len(occurrences),
            "is_discontinuous": any(gap > 25 for gap in gaps),
            "category_counts": dict(Counter(o["category"] for o in occurrences)),
            "occurrences": occurrences,
        })
    unassigned = []
    for cand in candidates:
        if cand["candidate_id"] not in assigned:
            unassigned.append({
                "candidate_id": cand["candidate_id"],
                "note_id": note_id,
                "unit_start": cand["unit_start"],
                "unit_end": cand["unit_end"],
                "category": cand["aspect"],
                "summary": cand["summary"],
                "source_excerpt": excerpt(cand["text"], 300),
            })
    return {
        "note_id": note_id,
        "source_path": str(source_path),
        "source_hash": source_hash,
        "memory_model": "thread_first_no_titles_v1",
        "description": "Product-shaped memory object: thread IDs with summaries and source-linked occurrences. Thread titles intentionally omitted.",
        "threads": memory_threads,
        "unassigned_observations": unassigned,
    }


def write_readable_memory(memory: dict[str, Any], path: Path) -> None:
    lines = [
        f"# Memory: {memory['note_id']}",
        "",
        "Thread-first product mock. Thread titles are intentionally omitted; summaries and source occurrences carry the meaning.",
        "",
    ]
    for thread in memory["threads"]:
        lines.append(f"## `{thread['memory_id']}`")
        lines.append(f"Status: {thread['status']} | Confidence: {thread['confidence']} | Occurrences: {thread['occurrence_count']} | Discontinuous: {thread['is_discontinuous']}")
        if thread.get("line_of_inquiry"):
            lines.append(f"Line of inquiry: {thread['line_of_inquiry']}")
        if thread.get("summary"):
            lines.append(f"Summary: {thread['summary']}")
        if thread.get("why_it_matters"):
            lines.append(f"Why it matters: {thread['why_it_matters']}")
        counts = ", ".join(f"{k}: {v}" for k, v in thread.get("category_counts", {}).items()) or "none"
        lines.append(f"Aspect counts: {counts}")
        lines.append("")
        for occ in thread["occurrences"][:4]:
            lines.append(f"- **{occ['category']}** | units {occ['unit_start']}-{occ['unit_end']} | {occ['summary']}")
            lines.append(f"  > {occ['source_excerpt']}")
        if len(thread["occurrences"]) > 4:
            lines.append(f"- ... {len(thread['occurrences']) - 4} more occurrence(s)")
        lines.append("")
    if memory["unassigned_observations"]:
        lines.append("## Unassigned observations")
        lines.append("")
        for item in memory["unassigned_observations"][:10]:
            lines.append(f"- `{item['candidate_id']}` | {item['category']} | units {item['unit_start']}-{item['unit_end']} | {item['summary']}")
        lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def process_transcript(config: dict[str, Any], source_path: Path) -> dict[str, Any]:
    note_id = slugify(source_path.name)
    run_dir = OUTPUTS_DIR / note_id
    run_dir.mkdir(parents=True, exist_ok=True)
    text = source_path.read_text(encoding="utf-8", errors="replace")
    source_hash = sha256_text(text)
    manifest_path = run_dir / "manifest.json"
    previous_manifest = read_json(manifest_path, {})
    if previous_manifest.get("source_hash") and previous_manifest.get("source_hash") != source_hash:
        print("  source changed since previous run; refreshing transcript checkpoints")
        for cache_dir in [run_dir / "checkpoints", run_dir / "debug"]:
            if cache_dir.exists():
                shutil.rmtree(cache_dir)
    print(f"Processing {source_path.name} as {note_id}")
    units = unitise(text)
    units_by_id = {u.unit_id: u for u in units}
    (run_dir / "numbered_units.md").write_text("\n".join(f"[{u.unit_id}] {u.text}" for u in units) + "\n", encoding="utf-8")
    batches = make_batches(units, int(config["batch_size_units"]), int(config["overlap_units"]))
    candidates = []
    failed_batches = []
    for idx, batch in enumerate(batches, start=1):
        rows, err = process_batch(config, note_id, idx, batch, run_dir, units_by_id)
        candidates.extend(rows)
        if err:
            failed_batches.append(err)
    candidates = sorted({c["candidate_id"]: c for c in candidates}.values(), key=lambda c: (c["unit_start"], c["unit_end"], c["candidate_id"]))
    threads, memory_error = synthesize_memory(config, note_id, candidates, run_dir)
    memory = build_memory_object(note_id, source_path, source_hash, threads, candidates)
    write_json(run_dir / "memory.json", memory)
    write_readable_memory(memory, run_dir / "MEMORY.md")
    write_jsonl(run_dir / "candidate_observations.jsonl", candidates)
    write_json(run_dir / "failed_batches.json", failed_batches)
    write_json(run_dir / "memory_error.json", memory_error or {})
    manifest = {
        "note_id": note_id,
        "source_path": str(source_path),
        "source_hash": source_hash,
        "unit_count": len(units),
        "batch_count": len(batches),
        "candidate_count": len(candidates),
        "memory_count": len(memory["threads"]),
        "failed_batch_count": len(failed_batches),
        "memory_error": bool(memory_error),
        "updated_at": now(),
        "main_output": str((run_dir / "MEMORY.md").relative_to(ROOT)),
    }
    write_json(manifest_path, manifest)
    print(f"  memories: {manifest['memory_count']} | candidates: {manifest['candidate_count']} | failed batches: {manifest['failed_batch_count']}")
    return memory


def memory_embedding_text(memory: dict[str, Any], thread: dict[str, Any]) -> str:
    occ_text = "\n".join(
        f"- {o['category']}: {o['summary']} :: {o['source_excerpt']}" for o in thread.get("occurrences", [])[:4]
    )
    return "\n".join([
        f"note_id: {memory['note_id']}",
        f"memory_id: {thread['memory_id']}",
        f"line_of_inquiry: {thread.get('line_of_inquiry','')}",
        f"summary: {thread.get('summary','')}",
        f"why_it_matters: {thread.get('why_it_matters','')}",
        f"occurrences:\n{occ_text}",
    ])


def rebuild_global_index(config: dict[str, Any], memories: list[dict[str, Any]]) -> list[dict[str, Any]]:
    INDEX_DIR.mkdir(parents=True, exist_ok=True)
    memory_rows = []
    embedding_rows_existing = {row.get("memory_id"): row for row in read_jsonl(INDEX_DIR / "memory_embeddings.jsonl")}
    embedding_rows = []
    for memory in memories:
        for thread in memory.get("threads", []):
            row = {
                "memory_id": thread["memory_id"],
                "note_id": memory["note_id"],
                "source_path": memory["source_path"],
                "line_of_inquiry": thread.get("line_of_inquiry", ""),
                "summary": thread.get("summary", ""),
                "why_it_matters": thread.get("why_it_matters", ""),
                "status": thread.get("status", ""),
                "confidence": thread.get("confidence", ""),
                "occurrence_count": thread.get("occurrence_count", 0),
                "is_discontinuous": thread.get("is_discontinuous", False),
                "category_counts": thread.get("category_counts", {}),
                "occurrences": thread.get("occurrences", []),
                "embedding_text": memory_embedding_text(memory, thread),
            }
            memory_rows.append(row)
            if config.get("embed_memories"):
                existing = embedding_rows_existing.get(row["memory_id"])
                embedding_hash = sha256_text(row["embedding_text"])
                if (
                    existing
                    and existing.get("embedding_hash") == embedding_hash
                    and not config.get("force_rerun_embeddings")
                ):
                    embedding_rows.append(existing)
                else:
                    print(f"  embedding {row['memory_id']}")
                    embedding_rows.append({
                        "memory_id": row["memory_id"],
                        "note_id": row["note_id"],
                        "embedding_hash": embedding_hash,
                        "embedding": embed(config, row["embedding_text"]),
                    })
    write_jsonl(INDEX_DIR / "memories.jsonl", memory_rows)
    if config.get("embed_memories"):
        write_jsonl(INDEX_DIR / "memory_embeddings.jsonl", embedding_rows)
    summary = {
        "memory_count": len(memory_rows),
        "embedding_count": len(embedding_rows),
        "note_count": len({m["note_id"] for m in memory_rows}),
        "updated_at": now(),
    }
    write_json(INDEX_DIR / "index_summary.json", summary)
    return memory_rows


def cosine(a: list[float], b: list[float]) -> float:
    if not a or not b or len(a) != len(b):
        return 0.0
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(y * y for y in b))
    if na == 0 or nb == 0:
        return 0.0
    return dot / (na * nb)


def normalise_term(term: str) -> str:
    term = term.lower().strip("'")
    if len(term) > 4 and term.endswith("s") and not term.endswith("ss"):
        term = term[:-1]
    return term


def request_terms(request: str) -> set[str]:
    terms = set()
    for raw in re.findall(r"[a-zA-Z][a-zA-Z0-9']{1,}", request):
        term = normalise_term(raw)
        if term == "ai" or (len(term) >= 4 and term not in REQUEST_STOPWORDS):
            terms.add(term)
    return terms


def text_terms(text: str) -> set[str]:
    return {normalise_term(raw) for raw in re.findall(r"[a-zA-Z][a-zA-Z0-9']{1,}", text)}


def matched_request_terms(request: str, memory_text: str) -> list[str]:
    wanted = request_terms(request)
    available = text_terms(memory_text)
    return sorted(term for term in wanted if term in available)


def has_pack_evidence(matches: list[str]) -> bool:
    return len(matches) >= 2


def infer_pack_tags(request: str) -> list[str]:
    text = request.lower()
    inferred = []
    checks = [
        ("workflow_discovery", ["workflow", "process", "use case", "problem"]),
        ("light_automation", ["automation", "automate", "ai"]),
        ("client_service", ["client", "customer", "advice", "service"]),
        ("tax", ["tax"]),
        ("planning", ["planning", "advice"]),
        ("intake", ["intake", "questionnaire", "answers"]),
        ("forms", ["form", "fact-find"]),
        ("reports", ["report", "risk assessment"]),
        ("documents", ["document", "folder", "drive", "file"]),
        ("marketing", ["marketing", "seo", "website", "blog", "keyword"]),
    ]
    for tag, needles in checks:
        if any(needle in text for needle in needles):
            inferred.append(tag)
    return inferred


def normalise_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [str(item) for item in value if str(item).strip()]
    return []


def expand_pack_brief(req: dict[str, Any]) -> dict[str, Any]:
    request = re.sub(r"\s+", " ", str(req.get("request") or req.get("need") or "")).strip()
    tags = []
    for tag in normalise_list(req.get("tags")) + infer_pack_tags(request):
        tag = normalise_term(tag.replace(" ", "_"))
        if tag and tag not in tags:
            tags.append(tag)
    signals = []
    for tag in tags:
        for signal in PACK_SIGNAL_LEXICON.get(tag, []):
            if signal not in signals:
                signals.append(signal)
    for signal in normalise_list(req.get("signals")) + normalise_list(req.get("keywords")):
        signal = re.sub(r"\s+", " ", signal).strip()
        if signal and signal not in signals:
            signals.append(signal)
    query_text = " ".join([request, " ".join(tags), " ".join(signals)]).strip()
    return {
        "request": request,
        "tags": tags,
        "signals": signals,
        "query_text": query_text,
    }


def opportunity_label(mem: dict[str, Any]) -> str:
    text = (mem.get("embedding_text") or "").lower()
    best_label = "General workflow or decision support"
    best_score = 0
    for item in OPPORTUNITY_LABELS:
        score = sum(1 for needle in item["needles"] if needle in text)
        if score > best_score:
            best_label = item["label"]
            best_score = score
    return best_label


def opportunity_angle(label: str) -> str:
    for item in OPPORTUNITY_LABELS:
        if item["label"] == label:
            return item["angle"]
    return "Light automation could help if this pattern repeats across enough real client work."


def opportunity_prompt(label: str, mem: dict[str, Any]) -> str:
    needles = []
    for item in OPPORTUNITY_LABELS:
        if item["label"] == label:
            needles = item["needles"]
            break
    occurrences = mem.get("occurrences", [])
    for occurrence in occurrences:
        text = ((occurrence.get("summary") or "") + " " + (occurrence.get("source_excerpt") or "")).lower()
        if any(needle in text for needle in needles):
            return occurrence.get("summary") or mem.get("summary", "")
    if occurrences:
        return occurrences[0].get("summary") or mem.get("summary", "")
    return mem.get("summary", "")


def retrieve_memories(config: dict[str, Any], request: str, max_memories: int) -> list[dict[str, Any]]:
    memories = read_jsonl(INDEX_DIR / "memories.jsonl")
    if not memories:
        return []
    if config.get("embed_memories") and (INDEX_DIR / "memory_embeddings.jsonl").exists():
        q = embed(config, request)
        embeds = {row["memory_id"]: row["embedding"] for row in read_jsonl(INDEX_DIR / "memory_embeddings.jsonl")}
        scored = []
        for mem in memories:
            matches = matched_request_terms(request, mem.get("embedding_text") or "")
            if has_pack_evidence(matches):
                scored.append((cosine(q, embeds.get(mem["memory_id"], [])), matches, mem))
        scored.sort(key=lambda x: x[0], reverse=True)
        selected = []
        for score, matches, mem in scored[:max_memories]:
            mem = dict(mem)
            mem["retrieval_score"] = round(float(score), 4)
            mem["matched_request_terms"] = matches
            selected.append(mem)
        return selected
    # fallback lexical overlap
    scored = []
    for mem in memories:
        matches = matched_request_terms(request, mem.get("embedding_text") or "")
        if has_pack_evidence(matches):
            scored.append((len(matches), matches, mem))
    scored.sort(key=lambda x: x[0], reverse=True)
    return [dict(mem, retrieval_score=score, matched_request_terms=matches) for score, matches, mem in scored[:max_memories]]


def make_pack_brief(
    request: str,
    pack_id: str | None = None,
    title: str | None = None,
    tags: list[str] | None = None,
    signals: list[str] | None = None,
    max_memories: int = 8,
    max_occurrences_per_memory: int = 4,
) -> dict[str, Any]:
    request = re.sub(r"\s+", " ", request).strip()
    if not request:
        raise SystemExit("Pack request is empty.")
    brief_id = pack_id or slugify(request[:80])
    return {
        "pack_id": brief_id,
        "title": title or brief_id.replace("-", " ").replace("_", " "),
        "request": request,
        "tags": tags or [],
        "signals": signals or [],
        "max_memories": max_memories,
        "max_occurrences_per_memory": max_occurrences_per_memory,
    }


def build_context_pack_from_request(config: dict[str, Any], req: dict[str, Any]) -> Path:
    pack_id = req.get("pack_id") or "context-pack"
    title = req.get("title") or pack_id
    brief = expand_pack_brief(req)
    request = brief["request"]
    max_memories = int(req.get("max_memories", 8))
    max_occurrences = int(req.get("max_occurrences_per_memory", 4))
    selected = retrieve_memories(config, brief["query_text"], max_memories)
    out_path = CONTEXT_PACKS_DIR / f"{pack_id}.md"
    CONTEXT_PACKS_DIR.mkdir(parents=True, exist_ok=True)
    lines = [
        f"# {title}",
        "",
        "This context pack was assembled from local thread memories across all processed transcripts.",
        "",
        "## Pack request",
        "",
        request,
        "",
        "## Lightweight pack brief",
        "",
        f"Tags: {', '.join(brief['tags']) or 'none'}",
        "",
        f"Retrieval signals: {', '.join(brief['signals'][:40]) or 'none'}",
        "",
        "## Use rule",
        "",
        "Treat summaries and 'why it matters' as provisional interpretation. Treat source excerpts and unit ranges as evidence. Do not present inferred interpretation as agreed fact.",
        "",
        "## Workflow / use-case leads",
        "",
    ]
    if not selected:
        if read_jsonl(INDEX_DIR / "memories.jsonl"):
            lines.append("No sufficiently relevant memories found in the processed transcripts.")
            lines.append("")
            lines.append("This usually means the request is about a topic that is not present in the stored memory layer. Add/process relevant transcripts, or ask for a pack that matches the current transcripts.")
        else:
            lines.append("No memories retrieved. Run transcript processing first.")
        lines.append("")
    else:
        grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
        for mem in selected:
            grouped[opportunity_label(mem)].append(mem)
        for label, items in grouped.items():
            source_ids = ", ".join(f"`{item['memory_id']}`" for item in items[:3])
            terms = sorted({term for item in items for term in item.get("matched_request_terms", [])})
            lines.append(f"### {label}")
            lines.append(f"Source memories: {source_ids}")
            lines.append(f"Automation angle: {opportunity_angle(label)}")
            lines.append(f"Source prompt: {excerpt(opportunity_prompt(label, items[0]), 320)}")
            if terms:
                lines.append(f"Matched request terms: {', '.join(terms)}")
            lines.append("")
    lines.extend([
        "## Retrieved memories",
        "",
    ])
    for idx, mem in enumerate(selected, start=1):
        lines.append(f"### Memory {idx}: `{mem['memory_id']}`")
        lines.append(f"Source note: `{mem['note_id']}` | Score: {mem.get('retrieval_score', 'n/a')} | Confidence: {mem.get('confidence','')}")
        if mem.get("matched_request_terms"):
            lines.append(f"Matched request terms: {', '.join(mem['matched_request_terms'])}")
        if mem.get("line_of_inquiry"):
            lines.append(f"Line of inquiry: {mem['line_of_inquiry']}")
        if mem.get("summary"):
            lines.append(f"Summary: {mem['summary']}")
        if mem.get("why_it_matters"):
            lines.append(f"Why it matters: {mem['why_it_matters']}")
        lines.append("")
        for occ in mem.get("occurrences", [])[:max_occurrences]:
            lines.append(f"- **{occ['category']}** | units {occ['unit_start']}-{occ['unit_end']} | {occ['summary']}")
            lines.append(f"  > {occ['source_excerpt']}")
        if len(mem.get("occurrences", [])) > max_occurrences:
            lines.append(f"- ... {len(mem.get('occurrences', [])) - max_occurrences} more occurrence(s) in this memory")
        lines.append("")
    lines.extend([
        "## What this pack is missing",
        "",
        "This prototype retrieves relevant memories but does not yet run an evidence-review/synthesis pass over the selected memories. Human review is still required before using this as a final context handoff.",
        "",
    ])
    out_path.write_text("\n".join(lines), encoding="utf-8")
    write_json(CONTEXT_PACKS_DIR / f"{pack_id}.json", {"pack_id": pack_id, "request": req, "expanded_brief": brief, "selected_memory_ids": [m["memory_id"] for m in selected], "created_at": now()})
    return out_path


def build_context_pack(config: dict[str, Any], pack_request_path: Path) -> Path:
    req = read_json(pack_request_path, {})
    if not req.get("pack_id"):
        req["pack_id"] = slugify(pack_request_path.name)
    return build_context_pack_from_request(config, req)


def process_all(config: dict[str, Any]) -> list[dict[str, Any]]:
    transcripts = sorted(p for p in TRANSCRIPTS_DIR.iterdir() if p.suffix.lower() in {".md", ".txt"})
    if not transcripts:
        print("No transcripts found in transcripts/.")
        return []
    memories = []
    for source_path in transcripts:
        memories.append(process_transcript(config, source_path))
        print("")
    print("Rebuilding global memory index")
    rebuild_global_index(config, memories)
    return memories


def build_all_packs(config: dict[str, Any]) -> list[Path]:
    requests = sorted(p for p in PACK_REQUESTS_DIR.iterdir() if p.suffix.lower() == ".json")
    paths = []
    for req_path in requests:
        print(f"Building context pack from {req_path.name}")
        path = build_context_pack(config, req_path)
        print(f"  wrote {path.relative_to(ROOT)}")
        paths.append(path)
    return paths


def write_root_readme_outputs() -> None:
    lines = [
        "# Output guide",
        "",
        "Main files to inspect:",
        "",
        "1. `outputs/<note_id>/MEMORY.md` - readable memory for one transcript.",
        "2. `index/memories.jsonl` - all transcript memories in one store.",
        "3. `index/memory_embeddings.jsonl` - embeddings for retrieval.",
        "4. `context_packs/*.md` - context packs built from memories across all transcripts.",
        "",
        "Debug files live under `outputs/<note_id>/debug/` and checkpoints under `outputs/<note_id>/checkpoints/`.",
        "",
        "Thread titles are intentionally omitted. The product-facing object uses memory IDs, summaries, line_of_inquiry, and source-linked occurrences.",
    ]
    (ROOT / "OUTPUTS.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Broccoli memory prototype")
    parser.add_argument("command", nargs="?", choices=["all", "ingest", "pack"], default="all")
    parser.add_argument("--request", help="Plain-language Day 2 context need. Builds one pack from the memory index.")
    parser.add_argument("--pack-id", help="Optional ID for a direct --request context pack.")
    parser.add_argument("--title", help="Optional title for a direct --request context pack.")
    parser.add_argument("--tag", action="append", default=[], help="Optional lightweight pack tag. Can be repeated.")
    parser.add_argument("--signal", action="append", default=[], help="Optional retrieval keyword/signal. Can be repeated.")
    parser.add_argument("--max-memories", type=int, default=8, help="Maximum memories to retrieve for a direct --request pack.")
    parser.add_argument(
        "--max-occurrences-per-memory",
        type=int,
        default=4,
        help="Maximum source occurrences shown per memory for a direct --request pack.",
    )
    args = parser.parse_args()
    start = datetime.now()
    print(f"Started at {start:%Y-%m-%d %H:%M:%S}\n")
    config = load_config()
    check_ollama(config)
    write_root_readme_outputs()
    if args.command in {"all", "ingest"}:
        process_all(config)
    if args.command in {"all", "pack"}:
        if not (INDEX_DIR / "memories.jsonl").exists():
            print("No global index found. Run ingest first.")
        elif args.request:
            brief = make_pack_brief(
                request=args.request,
                pack_id=args.pack_id,
                title=args.title,
                tags=args.tag,
                signals=args.signal,
                max_memories=args.max_memories,
                max_occurrences_per_memory=args.max_occurrences_per_memory,
            )
            print(f"Building context pack from direct request: {brief['pack_id']}")
            path = build_context_pack_from_request(config, brief)
            print(f"  wrote {path.relative_to(ROOT)}")
        else:
            build_all_packs(config)
    end = datetime.now()
    print(f"\nFinished at {end:%Y-%m-%d %H:%M:%S} (elapsed {end - start})")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nInterrupted.")
        sys.exit(130)
