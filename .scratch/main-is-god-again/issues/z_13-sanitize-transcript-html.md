---
title: Sanitize transcript HTML
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

How do we stop transcript markdown from becoming unsafe HTML in the webview (`{@html}` / wide markdown options), without breaking normal transcript display?

**Done when:** User-influenced transcript text cannot inject HTML/handlers into the UI; approach recorded (sanitize, safer render, or both).

## Spec (to-spec)

Evidence: security S2 (`research/security-review.md`) — `HistoryController::render_transcript_html` uses `pulldown_cmark::Options::all()` then `html::push_html` (`src-tauri/src/controllers/history.rs`); frontend injects with `{@html html}` (`TranscriptPanel.svelte`). Speaker-block UI path already uses plain Svelte text (`{block.text}`); the HTML path is the fallback when there are no speaker blocks (plain segments, legacy `.md`, etc.). CSP `script-src 'self'` does not stop event-handler / navigation injection. Combined with flat IPC (ticket 16), this is an escalation path.

**In this ticket:** make the HTML the panel injects safe.  
**Not this ticket:** per-window IPC allowlists → *Least-privilege IPC* (16). Legacy Keychain purge → (14). Model SHA → (15). ADR wording → (18).

### What the panel actually needs

In-app body from `render_markdown` is mostly plain paragraphs, sometimes `**bold**` labels in the speaker-body markdown helper. Legacy `.md` may have richer markdown (headings, lists). We do **not** need raw HTML, math, footnotes, or “every GFM knob” for this panel.

### Aggression (agreed)

**(2) Sanitize after markdown** — Keep markdown → HTML → `{@html}`, but run the string through ammonia with a tight allowlist. Also stop using `Options::all()`.

### Code cut (fixed)

1. Change `render_transcript_html` so unsafe tags/attrs cannot reach the webview.
2. Add regression tests: script / `onerror` / `javascript:` must not survive; normal paragraphs + bold still work.
3. No frontend redesign of TranscriptPanel.

### Done when

1. `Options::all()` gone from this path.
2. Crafted malicious markdown/HTML cannot produce injectable handlers in `note_render_transcript_html` output.
3. Normal transcript still displays readable paragraphs.
4. `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` pass.
5. Approach recorded in Resolution.

## Resolution

Option **(2)** on `feature/0.3/embeds`.

| Cut | Result |
|---|---|
| `Options::all()` | Replaced with `Options::empty()` in `markdown_to_safe_html` |
| Post-render scrub | `ammonia` allowlist: `p`/`br`/`strong`/`em`/`b`/`i`/`ul`/`ol`/`li`/`h1`–`h3`/`a`/`blockquote`/`code`/`pre`/`hr`; links get `rel="noopener noreferrer"` |
| IPC / panel | Unchanged — still `{@html}` of sanitized HTML |
| Tests | Keep emphasis; strip script/`onerror`/`javascript:`; segment payload through controller |

**Verify:** `cargo test -p ScribeFloat` → 336 passed, 5 ignored; `cargo clippy -p ScribeFloat -- -D warnings` clean. Grep: no `Options::all()` under `src-tauri/src/`.

## Comments

- 2026-07-19: claimed; to-spec drafted; human chose sanitize (2); implemented and closed.
