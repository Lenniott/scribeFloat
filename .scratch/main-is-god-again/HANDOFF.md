# Handoff — Main is God again

**When:** 2026-07-19 (session closed tickets 14–15; 13–15 uncommitted)  
**Branch:** `feature/0.3/embeds` → merge into `main` **untagged**  
**Open a new chat and say:**  
> Continue from `.scratch/main-is-god-again/HANDOFF.md` — `/wayfinder` on [Main is God again](./MAP.md)

This file is the session bridge. Prefer this path over `/var` or `$TMPDIR`.

Map / out of scope: [MAP.md](./MAP.md)  
Parked debt: [KNOWN-ISSUES.md](./KNOWN-ISSUES.md)  
Research (closed): [research/](./research/)

---

## Required session ritual — `/to-spec` first

**Do not jump to `/implement` or start coding a merge-blocker ticket cold.**

For **every** remaining frontier ticket (**16–18**, and any new merge-blocker):

1. Claim `assignee` on the ticket.
2. Run **`/to-spec`** (or draft a `## Spec (to-spec)` on the ticket the same way [08](./issues/08-delete-dead-multi-model-paths.md)–[15](./issues/15-verify-all-bundled-models-before-load.md) did).
3. Put the spec on the ticket; get human agreement on any open choice (aggression, scope, error behaviour).
4. Only then implement.

Skipping `/to-spec` was a failure mode in earlier sessions — treat it as **mandatory**, not optional.

---

## Where this session left the tree

**Uncommitted (do not invent a different story):**

| Area | State |
|------|--------|
| Ticket 13 | `ammonia` + `markdown_to_safe_html` in `history.rs`; `Cargo.toml` / `Cargo.lock` |
| Ticket 14 | `lib.rs` always `delete_voice_crypto_key()`; purge comments |
| Ticket 15 | `bundled_models.rs`; Sortformer SHA + ensure-before-load; ModelService/DiarizationService resource_dir heal; startup seed for VAD/Sortformer hash heal |
| Scratch | Tickets 13–15 closed; MAP / KNOWN-ISSUES / HANDOFF updated |

**Already on remote tip** (before 13+ edits): thin-docs, Float UI cut, multi-model delete, bundle-only models. Confirm with `git status -sb` / `git log`.

**Verify already run through ticket 15:** `cargo test -p ScribeFloat` → 344 passed, 5 ignored; `cargo clippy -p ScribeFloat -- -D warnings` clean. Re-run after any further edit.

**Commit:** only if human asks. Natural batch: 13+14+15 + scratch, or split by ticket.

---

## Closed this stretch (do not re-litigate)

Detail lives on each ticket’s Resolution.

- [Finish the thin-docs cut](./issues/01-finish-thin-docs-cut.md) — **committed**
- [Remove Float coming-soon from shipped UI](./issues/07-remove-float-coming-soon-ui.md) — **committed**
- [Delete dead multi-model paths](./issues/08-delete-dead-multi-model-paths.md) — **committed** (with models work)
- [Bundle-only models — no runtime downloads](./issues/12-bundle-only-models-no-runtime-fetch.md) — **committed**
- [Sanitize transcript HTML](./issues/13-sanitize-transcript-html.md) — **implemented, not committed**
- [Always delete legacy voice Keychain key](./issues/14-always-delete-legacy-voice-keychain-key.md) — **implemented, not committed**; voiceprint topic closed for this map
- [Verify all bundled models before load](./issues/15-verify-all-bundled-models-before-load.md) — aggression **(2)** offline re-seed + hash — **implemented, not committed**
- Sort / reviews (06 + research) — source of the ordered blocker list

---

## Frontier next (unblocked)

Work **one** ticket per session. Claim → **`/to-spec`** → human OK → implement.

1. [Least-privilege IPC per window](./issues/16-least-privilege-ipc-per-window.md) ← start here
2. [Unify Record and Dictate naming and seams](./issues/17-unify-record-dictate-naming-and-seams.md) — human elevated; grill scope in/before to-spec
3. [Mark and amend ADRs for reality](./issues/18-mark-and-amend-adrs-for-reality.md)

Also unblocked: [Write the forward working method](./issues/02-write-forward-working-method.md).

Silicon smoke ([09](./issues/09-silicon-ship-bar-smoke.md)) waits on **13–18** (13–15 done in tree; still need commit + the rest closed).

---

## After those

Silicon ship-bar smoke → Merge spine into main untagged → Delete stale branches

---

## Push incident (resolved — remember if it recurs)

Push once failed: GitHub rejected history containing `tests/mic.wav` (~171 MB). Blob lived in **unpushed** history after the file was removed from the tree. Fix = rewrite unpushed commits to drop `tests/*.wav` (gitignore already has `/tests/*.wav`). Explain in plain language to the human before rewriting.

---

## Human hard preference

**Do not recreate `skills/new-adr` or `skills/new-story`.** Human deleted them repeatedly; a prior commit-curator “restore” brought them back. README/AGENTS say they are retired. Capture work via `.scratch/` + `docs/agents/issue-tracker.md`; write ADRs as plain files under `docs/adr/`.

**Voiceprint never shipped / do not re-litigate.** Exploration-only; ticket **14** closed. Do not bring voiceprint up again. Canonical line: [MAP.md](./MAP.md) Decisions.

---

## How to talk to the human

Use **plain language**. Do not compress jargon or assume they already hold review context. Split bundled findings into separate questions. Prefer common words over ticket-speak in chat.

---

## Suggested skills

| Skill | When |
|-------|------|
| **`/to-spec` (required)** | **First action after claiming any of 16–18** — write `## Spec (to-spec)` on the ticket; wait for human if choices remain. Do not code first. |
| Wayfinder / `docs/agents/issue-tracker.md` | Claim, resolve, map gist under Decisions so far |
| Security-review (`review-security` / security-review) | Ticket **16** — evidence + rubric; still **after** `/to-spec` |
| Grilling | Ticket **17** scope (before or as part of to-spec) |
| Domain modeling | Ticket **18** ADR wording |
| Design skill / UI enforcement | Only if a ticket touches Svelte chrome |
| Commit curator | When human asks to commit — Turn 1 plan only; **never restore deleted skills** |

---

## Stance / do not

Unease + real finding = merge-blocker. “Just get it done” is not a resolution.

- No release tag / website publish this map
- No Upload redesign beyond honesty
- No knowledge / embeddings / retrieval rebuild
- Do not recreate cut doc trees without ADR + human OK
- Do not recreate `new-adr` / `new-story` skills
- **Do not implement merge-blockers without `/to-spec` on the ticket first**
- Do not commit unless asked
- Do not re-litigate voiceprint / invent released-user blast radius
