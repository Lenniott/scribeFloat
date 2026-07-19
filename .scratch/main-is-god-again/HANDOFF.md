# Handoff — Main is God again

**When:** 2026-07-19 (tickets 16 + 18 committed; ready for Silicon ship-bar smoke)  
**Branch:** `feature/0.3/embeds` → merge into `main` **untagged**  
**HEAD:** confirm with `git log -1` / `git status -sb` after pull  
**Open a new chat and say:**  
> Continue from `.scratch/main-is-god-again/HANDOFF.md` — run [Silicon ship-bar smoke](./issues/09-silicon-ship-bar-smoke.md) on Apple Silicon

This file is the session bridge. Prefer this path over `/var` or `$TMPDIR`.

Map / out of scope: [MAP.md](./MAP.md)  
Parked debt: [KNOWN-ISSUES.md](./KNOWN-ISSUES.md)  
Research (closed): [research/](./research/)

---

## Frontier next — Silicon smoke (human + agent)

**Ticket:** [Silicon ship-bar smoke](./issues/09-silicon-ship-bar-smoke.md)  
**Platform:** Apple Silicon only (this map’s confidence bar).  
**Blockers for smoke:** all closed (**16**, **18**). Ticket **17** is Known issues — not required.

This is a **live product walk**, not a code ticket. Agent claims the ticket, guides the checklist, records pass/fail on the ticket + map Decisions. Failures → fix as new merge-blocker **or** park in Known issues with human OK.

### Before you start

1. Restart the app so new IPC capabilities load (`capabilities/dictate|onboarding|shell.json`). Old `default.json` is gone.
2. Prefer a cold-ish launch (quit fully, relaunch). Dev: `npm run tauri dev` or your usual Silicon run.
3. Bundled models: release/dev with real model files in resources; empty placeholders skip seeding — smoke needs real Whisper/VAD/Sortformer for a full pass.

### Ship-bar checklist (ticket Question)

Record pass/fail per step on the ticket:

1. **First-run / permissions** — onboarding (or reset onboarding from Settings if already done) → mic (+ optional Accessibility / Input Monitoring for Dictate paste).
2. **Dictate once** — hotkey → speak → release → text pastes or lands on clipboard; quick Note appears in history/Notes.
3. **Record once** — New Note / Record → stop → transcript on the Note; speaker labels if Sortformer available.
4. **Notes** — note visible with transcript; open it; content looks sane (no XSS junk if you paste weird markdown — ticket 13).
5. **Speaker rename** — rename a speaker on that Note; turns for that speaker update.
6. **Relaunch** — quit → open again → same Note still there.

Optional smoke notes (not blockers unless they fail the bar):

- Satellite windows still work after ACL (Dictate overlay, onboarding if shown).
- No Settings → Models; errors say reinstall / bundled models, not download.
- Offline: no surprise network fetch for models.

### After smoke

- Append `## Resolution` on ticket **09** (pass/fail table); close if pass (or human OK with parked fails).
- Map Decisions gist + update this HANDOFF.
- Next: [Write the forward working method](./issues/02-write-forward-working-method.md) → [Merge spine into main untagged](./issues/10-merge-spine-into-main-untagged.md) → [Delete stale branches](./issues/11-delete-stale-branches.md).

---

## Session ritual (smoke vs code tickets)

**Smoke (09):** no `/to-spec` required — execute the checklist, write results.  
**New merge-blocker found during smoke:** claim → `/to-spec` → human OK → implement (same ritual as 13–16 / 18).  
**Do not** jump straight to coding a new blocker cold.

---

## Where the tree is

| Area | State |
|------|--------|
| Ticket **16** | B+ least-privilege IPC — capabilities split; `permissions/README.md`; ACL tests |
| Ticket **18** | School 1 ADR stamps + ADR-0010 one-model amend |
| Ticket **17** | Demoted → Known issues (later wayfinder) |
| Verify | `cargo test -p ScribeFloat` → 349 passed, 5 ignored; `cargo clippy -p ScribeFloat -- -D warnings` clean (re-run if tree dirty) |

**Push:** only if human asks.  
**Commit:** only if human asks.

---

## Closed merge-blockers (do not re-litigate)

- Thin-docs, Float UI cut, multi-model delete, bundle-only models
- [Sanitize transcript HTML](./issues/13-sanitize-transcript-html.md)
- [Always delete legacy voice Keychain key](./issues/14-always-delete-legacy-voice-keychain-key.md) — voiceprint topic **closed**
- [Verify all bundled models before load](./issues/15-verify-all-bundled-models-before-load.md)
- [Least-privilege IPC per window](./issues/16-least-privilege-ipc-per-window.md) — B+
- [Mark and amend ADRs for reality](./issues/18-mark-and-amend-adrs-for-reality.md) — School 1
- Sort / reviews (06 + research)

---

## Context the next agent must not rediscover

- **Voiceprint never shipped** — local hygiene only; do not invent released-user blast radius.
- **SHA-256:** pack pin in fetch script + runtime trust of writable `{app_data}/models` (ticket 15 heal from *installed* app resources).
- **IPC:** new `#[tauri::command]` → `generate_handler` + `APP_COMMANDS` in `build.rs` + right set in `src-tauri/permissions/sets/` (see `permissions/README.md`). Ids are kebab (`allow-scribe-start`).
- **ADRs:** School 1 — binding / aspirational / superseded + Wayfinder provenance. Aspirational stays in `docs/adr/`.
- **Known issues:** Record/Dictate naming honesty (ex-17); spoken triggers as Dictate-only later; other parked S7–S12 items.

---

## Push incident (resolved — remember if it recurs)

Push once failed: GitHub rejected history containing `tests/mic.wav` (~171 MB). Fix = rewrite **unpushed** commits to drop `tests/*.wav`. Explain in plain language before rewriting.

---

## Human hard preference

**Do not recreate `skills/new-adr` or `skills/new-story`.** Capture via `.scratch/` + `docs/agents/issue-tracker.md`; ADRs as plain files under `docs/adr/`.

**Voiceprint never shipped / do not re-litigate.**

---

## How to talk to the human

Plain language. One checklist question at a time if something fails. Prefer common words over ticket-speak.

---

## Suggested skills

| Skill | When |
|-------|------|
| Wayfinder / issue-tracker | Claim **09**, write Resolution, map gist |
| `/to-spec` | Only if smoke finds a **new** merge-blocker |
| Design / UI enforcement | Only if a fix touches Svelte chrome |
| Commit curator | When human asks to commit — **never restore deleted skills** |

---

## Stance / do not

Unease + real finding = merge-blocker. “Just get it done” is not a resolution.

- No release tag / website publish this map
- No Upload redesign beyond honesty
- No knowledge / embeddings / retrieval rebuild
- Do not recreate cut doc trees without ADR + human OK
- Do not recreate `new-adr` / `new-story` skills
- Do not implement new merge-blockers without `/to-spec` first
- Do not commit unless asked
- Do not re-litigate voiceprint
- Do not block smoke on ticket **17** naming debt
