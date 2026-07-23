#!/usr/bin/env python3
"""Query tool for ScribeFloat design knowledge files.

Run from any directory — paths are resolved relative to this script.
See SKILL.md for full usage examples.
"""

import json
import sys
from pathlib import Path

DIR = Path(__file__).parent
UX_FILE = DIR / "ux.scribefloat.json"
DS_FILE = DIR / "design-system.json"


def load_ux():
    with open(UX_FILE) as f:
        return json.load(f)


def load_ds():
    with open(DS_FILE) as f:
        return json.load(f)


# ── UX helpers ────────────────────────────────────────────────────────────────

def _print_table(rows):
    if not rows:
        return
    print()
    for row in rows:
        print(" | ".join(str(c) for c in row))


def _print_items(items):
    for item in items:
        if not isinstance(item, dict):
            continue
        title = item.get("title", "")
        body = item.get("content", item.get("text", ""))
        if title:
            print(f"\n**{title}** {body}")
        else:
            print(f"- {body}")


def _print_section(sec):
    print(f"\n### {sec['title']}")
    if sec.get("content"):
        print(f"\n{sec['content']}")
    _print_items(sec.get("items", []))
    _print_table(sec.get("table"))


def _print_chapter(ch):
    print(f"\n## {ch['title']}")
    if ch.get("content"):
        print(f"\n{ch['content']}")
    _print_items(ch.get("items", []))
    for sec in ch.get("sections", []):
        _print_section(sec)
    _print_table(ch.get("table"))


def ux_toc(data):
    print(f"# {data['title']}\n")
    for ch in data.get("chapters", []):
        print(f"  {ch['title']}")
        for sec in ch.get("sections", []):
            print(f"    └─ {sec['title']}")


def ux_chapter(data, name):
    name_lower = name.lower()
    for ch in data.get("chapters", []):
        if name_lower in ch["title"].lower():
            _print_chapter(ch)
            return
    print(f"No chapter matching '{name}'. Available:")
    for ch in data.get("chapters", []):
        print(f"  {ch['title']}")


def ux_section(data, name):
    name_lower = name.lower()
    for ch in data.get("chapters", []):
        for sec in ch.get("sections", []):
            if name_lower in sec["title"].lower():
                print(f"[From: {ch['title']}]")
                _print_section(sec)
                return
    print(f"No section matching '{name}'.")


def ux_all(data):
    print(f"# {data['title']}\n")
    if data.get("content"):
        print(data["content"])
    for ch in data.get("chapters", []):
        _print_chapter(ch)


def ux_search(data, term):
    t = term.lower()
    print(f"UX search: '{term}'\n")
    for ch in data.get("chapters", []):
        if t in json.dumps(ch).lower():
            print(f"  Chapter: {ch['title']}")
            for sec in ch.get("sections", []):
                if t in json.dumps(sec).lower():
                    print(f"    └─ {sec['title']}")


# ── DS helpers ────────────────────────────────────────────────────────────────

def ds_toc(data):
    print("# Design System — Table of Contents\n")
    for top in data:
        if top.startswith("$"):
            continue
        print(f"[{top}]")
        val = data[top]
        if isinstance(val, dict):
            for sub in val:
                print(f"  {top}.{sub}")
        print()


def ds_get(data, path):
    parts = path.split(".")
    obj = data
    for part in parts:
        if isinstance(obj, dict) and part in obj:
            obj = obj[part]
        else:
            print(f"Path '{path}' not found in design-system.json.")
            return
    print(json.dumps(obj, indent=2))


def _search_obj(obj, term, path=""):
    results = []
    t = term.lower()
    if isinstance(obj, dict):
        for k, v in obj.items():
            p = f"{path}.{k}" if path else k
            if t in str(k).lower() or (isinstance(v, str) and t in v.lower()):
                results.append(p)
            results.extend(_search_obj(v, term, p))
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            results.extend(_search_obj(item, term, f"{path}[{i}]"))
    return results


def ds_search(data, term):
    print(f"DS search: '{term}'\n")
    paths = _search_obj(data, term)
    seen = set()
    for p in paths:
        short = ".".join(p.split(".")[:3])
        if short not in seen:
            seen.add(short)
            print(f"  {short}")


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    args = sys.argv[1:]
    if not args:
        print("Usage: python3 query.py <target> <command> [args]")
        print("See SKILL.md for full usage.")
        return

    target = args[0]

    if target == "search":
        term = " ".join(args[1:])
        ux_search(load_ux(), term)
        print()
        ds_search(load_ds(), term)
        return

    if target == "ux":
        data = load_ux()
        cmd = args[1] if len(args) > 1 else "toc"
        rest = " ".join(args[2:])
        if cmd == "toc":
            ux_toc(data)
        elif cmd == "chapter":
            ux_chapter(data, rest)
        elif cmd == "section":
            ux_section(data, rest)
        elif cmd == "all":
            ux_all(data)
        elif cmd == "search":
            ux_search(data, rest)
        else:
            print(f"Unknown ux command: {cmd}")
        return

    if target == "ds":
        data = load_ds()
        cmd = args[1] if len(args) > 1 else "toc"
        rest = args[2] if len(args) > 2 else "meta"
        if cmd == "toc":
            ds_toc(data)
        elif cmd == "get":
            ds_get(data, rest)
        elif cmd == "search":
            ds_search(data, " ".join(args[2:]))
        else:
            print(f"Unknown ds command: {cmd}")
        return

    print(f"Unknown target: '{target}'. Use: ux | ds | search")


if __name__ == "__main__":
    main()
