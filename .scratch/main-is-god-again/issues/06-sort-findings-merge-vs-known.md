---
title: Sort findings into merge-blockers vs Known issues
labels: [wayfinder:grilling]
status: closed
assignee: cursor-agent
blocked_by:
  - "03-adr-reality-audit.md"
  - "04-security-review-with-rubric.md"
  - "05-architecture-single-model-review.md"
parent: MAP.md
---

## Question

Given the three review findings files, which items are merge-blockers for *Main is God again*, and which go into `KNOWN-ISSUES.md`?

Default: human unease + real evidence ⇒ blocker. Outcome is an agreed ordered list of follow-on work (still decision/planning — implementation tickets come after `/to-spec` unless a finding is trivial and already scoped as a wayfinder task).

## Sorting (agreed)

| # | Finding | Bucket | Notes |
|---|---------|--------|-------|
| S1 | Startup VAD / any runtime model fetch vs PRIVACY | **merge-blocker** | Bundle-only; no Hugging Face at runtime |
| S2 | Unsanitized transcript HTML (`{@html}`) | **merge-blocker** | Sanitize or stop `{@html}` |
| S3 | Legacy biometric fields in `history.jsonl` until compact | **Known issues** | Smoke / wipe test data |
| S4 | Orphaned Keychain voice crypto key | **merge-blocker** | Always delete — voiceprint “as if never happened” |
| S5 | Sortformer runtime integrity vs Whisper/VAD | **merge-blocker** | Hash or re-seed from signed bundle |
| S6 | Flat IPC — every window can invoke every command | **merge-blocker** | Least-privilege capabilities |
| S7 | Transcribe/Upload arbitrary audio paths | **Known issues** | |
| S8 | `transcribe_open_output` any `.md` | **Known issues** | |
| S9 | Broad opener/dialog defaults | **Known issues** | |
| S10 | Windows open_file / open_with_app_path | **Known issues** | |
| S11 | Accessibility / auto-paste default on | **Known issues** | |
| S12 | Native ML / dependency hotspots | **Known issues** | |
| A1 | Product/docs still sell multi-model / fast–refined | **merge-blocker** | |
| A2 | Dead multi-model selection machinery | **merge-blocker** | Ticket 08 |
| A3 | Dual Record/Dictate controllers + “Scribe” naming | **merge-blocker** | Human elevated |
| A4 | Unfinished Notes UI leftovers | **Known issues** | |
| A5 | Speaker rename edge cases | **Known issues** | |
| A6 | Dual audio vs “4+4 speakers” idea | **Known issues** | |
| A7 | Dictate silence-chunk early transcription idea | **Known issues** | |
| A8 | ADR-0010 still fast vs refined / Models UI | **merge-blocker** | |
| A9 | Skills/plans mention deleted Models UI | **Known issues** | |
| A10 | Unbuilt ADRs still look “done” | **merge-blocker** | Mark aspirational |

## Resolution

Human confirmed the bucket list. Known issues live in [`KNOWN-ISSUES.md`](../KNOWN-ISSUES.md).

### Ordered follow-on work (merge-blockers)

Do these before Silicon smoke / merge. Prefer `/to-spec` per ticket; do not “just get it done.”

1. [Delete dead multi-model paths](./08-delete-dead-multi-model-paths.md) — code + error strings (A2; product copy coordinated with #12)
2. [Bundle-only models — no runtime downloads](./12-bundle-only-models-no-runtime-fetch.md) — S1 + honest PRIVACY/README/site/CONTEXT (A1)
3. [Sanitize transcript HTML](./13-sanitize-transcript-html.md) — S2
4. [Always delete legacy voice Keychain key](./14-always-delete-legacy-voice-keychain-key.md) — S4
5. [Verify all bundled models before load](./15-verify-all-bundled-models-before-load.md) — S5
6. [Least-privilege IPC per window](./16-least-privilege-ipc-per-window.md) — S6
7. [Unify Record and Dictate naming and seams](./17-unify-record-dictate-naming-and-seams.md) — A3
8. [Mark and amend ADRs for reality](./18-mark-and-amend-adrs-for-reality.md) — A8 + A10

Then: *Silicon ship-bar smoke* → *Write the forward working method* → *Merge spine into main untagged* → *Delete stale branches*.

## Comments

- 2026-07-19: grilled and closed; ticket file restored mid-session after accidental deletion.
