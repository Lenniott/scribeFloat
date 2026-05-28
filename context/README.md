# ScribeFloat — context & design docs

Entry point for **what the app does** and **how it is built**. For build commands, layer enforcement, and debugging playbooks, see [`CLAUDE.md`](../CLAUDE.md) at the repo root.

---

## What ScribeFloat is

A local-first desktop app (Tauri + Svelte) with three workflows:

| Workflow | Purpose |
|----------|---------|
| **Scribe** | Record mic (+ optional speaker capture), transcribe to Markdown |
| **Dictate** | Hotkey voice input → paste or clipboard |
| **Transcribe** | Import existing audio files |

Whisper runs on-device. The only routine network use is optional model download from Hugging Face.

---

## Reading order

Read in this order before changing behaviour or architecture:

1. **[architecture.md](architecture.md)** — C4 diagrams, layer rules, service ownership, module map
2. **[action-flows.md](action-flows.md)** — Step-by-step flows (source of truth for *behaviour*)
3. **[componets.md](componets.md)** — UI component catalogue
4. **[design-skill/SKILL.md](design-skill/SKILL.md)** — Design tokens and UX playbook (query before writing Tailwind)
5. **[../docs/remaining-work.md](../docs/remaining-work.md)** — Deferred follow-ups from the Intel perf branch

Compliance and data handling: **[../PRIVACY.md](../PRIVACY.md)**.

---

## Behaviour quick reference (current)

These details change often — if code and docs disagree, **fix the doc or confirm the code change was intentional**.

| Topic | Current behaviour |
|-------|-------------------|
| Default save folder | `~/Documents/transcripts_scribefloat/` (configurable) |
| Transcripts | Save folder **root**: `{title}_{model}.md`; `_1`, `_2`, … on collision |
| Scribe WAVs | Staging dir `{save_folder}/{timestamp}/`; removed when **Keep audio** is off |
| Capture I/O | `AudioService` streams checkpointed WAV during recording |
| Durable files | `OutputService` — transcripts, manifests, cleanup, dictate history, failure salvage |
| Dictate audio | Temp WAV in app data (`dictate_temp/`); deleted on success; salvaged on error |
| Model perf | Cached Whisper contexts; tiny/base preloaded when recording starts |
| macOS releases | Separate Apple Silicon (Metal) and Intel (AVX2) native builds |

---

## Repo layout (short)

```
src/                 SvelteKit UI — lib/screens/, lib/components/
src-tauri/src/       Rust backend — commands → controllers → services → platform/
context/             This folder — architecture and UX reference
docs/                Backlog notes (remaining-work.md)
```

Full module map: [architecture.md § Level 4](architecture.md#level-4--code-key-module-map).

---

## When to update these docs

Update **action-flows.md** when user-visible workflow steps change. Update **architecture.md** when layers, services, or ownership rules change. Update this README only when the reading order or the quick-reference table needs a new row.
