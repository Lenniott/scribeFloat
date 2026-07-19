# Handoff — Main is God again

**When:** 2026-07-19 (ticket 12 closed)  
**Branch:** `feature/0.3/embeds` → merge into `main` **untagged**  
**Open a new chat and say:**  
> Continue from `.scratch/main-is-god-again/HANDOFF.md` — `/wayfinder` on [Main is God again](./MAP.md)

This file is the session bridge. Do not write a second handoff under `/var` or `$TMPDIR`.

Map / out of scope: [MAP.md](./MAP.md)  
Parked debt: [KNOWN-ISSUES.md](./KNOWN-ISSUES.md)  
Research (closed): [research/](./research/)

---

## Closed

- [Finish the thin-docs cut](./issues/01-finish-thin-docs-cut.md) — keep-set applied; **spine commit of doc deletions still not made** (plan in ticket resolution)
- [Remove Float coming-soon from shipped UI](./issues/07-remove-float-coming-soon-ui.md)
- [Sort findings into merge-blockers vs Known issues](./issues/06-sort-findings-merge-vs-known.md) — buckets + follow-on tickets **12–18**
- [Delete dead multi-model paths](./issues/08-delete-dead-multi-model-paths.md) — full collapse of chooser-shaped backend; tests + clippy green
- [Bundle-only models — no runtime downloads](./issues/12-bundle-only-models-no-runtime-fetch.md) — no runtime HF; hard VAD; public docs honest

**Working tree:** docs cut + Float UI + multi-model path deletion + bundle-only models are dirty; `.scratch/` is local/untracked. Do not commit unless the human asks.

---

## Frontier next (unblocked tasks)

Work **one** ticket per session; claim `assignee` first. Prefer `/to-spec` before big implementation.

1. [Sanitize transcript HTML](./issues/13-sanitize-transcript-html.md)
2. [Always delete legacy voice Keychain key](./issues/14-always-delete-legacy-voice-keychain-key.md)
3. [Verify all bundled models before load](./issues/15-verify-all-bundled-models-before-load.md)
4. [Least-privilege IPC per window](./issues/16-least-privilege-ipc-per-window.md)
5. [Unify Record and Dictate naming and seams](./issues/17-unify-record-dictate-naming-and-seams.md) — human elevated; scope may need grilling
6. [Mark and amend ADRs for reality](./issues/18-mark-and-amend-adrs-for-reality.md)

Also unblocked: [Write the forward working method](./issues/02-write-forward-working-method.md) — can run in parallel with care.

Ordered blocker list: ticket 06 **Resolution**. Silicon smoke (`09`) waits on **13–18** (08 + 12 done).

---

## After those

Silicon ship-bar smoke → Merge spine into main untagged → Delete stale branches

---

## How to talk to the human

Use **plain language**. Do not compress jargon or assume they already hold review context. Split bundled findings into separate questions. Prefer common words over ticket-speak in chat.

---

## Suggested skills

| Skill | When |
|-------|------|
| Wayfinder / `docs/agents/issue-tracker.md` | Claim, resolve, map gist |
| `/to-spec` (or equivalent) | Before implementing 13–18 |
| Grilling | Ticket 17 scope (how far to unify Record/Dictate) |
| Domain modeling | One-model glossary / ADR wording (18) |
| Security-review skill | Implementing 13, 14, 16 — evidence + rubric |
| Design skill / UI enforcement | Only if a ticket touches Svelte chrome |
| Commit curator | When human asks to commit |

---

## Stance / do not

Unease + real finding = merge-blocker. “Just get it done” is not a resolution.

- No release tag / website publish this map
- No Upload redesign beyond honesty
- No knowledge / embeddings / retrieval rebuild
- Do not recreate cut doc trees without ADR + human OK
- Do not commit unless asked
