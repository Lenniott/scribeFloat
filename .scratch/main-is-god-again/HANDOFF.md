# Handoff — Main is God again

**When:** 2026-07-23 — merge-blockers **19** / **20** / **21** closed; next = forward method **02**  
**Branch:** `feature/0.3/embeds` → merge into `main` **untagged**  
**HEAD tip (last commit):** confirm with `git log -1` / `git status -sb`  
**Open a new chat and say:**  
> Continue from `.scratch/main-is-god-again/HANDOFF.md` — claim + write [Write the forward working method](./issues/02-write-forward-working-method.md); then [Merge spine into main untagged](./issues/10-merge-spine-into-main-untagged.md)

This file is the session bridge. Prefer this path over `/var` or `$TMPDIR`.

Map / out of scope: [MAP.md](./MAP.md)  
Parked debt: [KNOWN-ISSUES.md](./KNOWN-ISSUES.md) — smoke parks audited 2026-07-21/23; nothing missing from that walk  
Research (closed): [research/](./research/)

---

## Frontier next

1. [Write the forward working method](./issues/02-write-forward-working-method.md)
2. [Merge spine into main untagged](./issues/10-merge-spine-into-main-untagged.md) (`blocked_by`: 02, 09✓, 19–21✓)
3. [Delete stale branches](./issues/11-delete-stale-branches.md)

---

## Smoke bar (closed 2026-07-21)

Ticket **09** closed with Resolution. Installed Silicon `.app` walk; relaunch also fine in `tauri dev` (file-backed Notes + `history.jsonl`, not a DB).

| Step | Status |
|------|--------|
| Preflight / real models | Pass |
| First-run / permissions | Fail → **19** ✓ |
| Dictate once | Capture pass; Continue → **20** ✓; Spaces/overlay → Known issues |
| Record once | Pass |
| Notes | Read pass; “Selection deleted” → **21** ✓ |
| Speaker rename | Pass (cascade); this-vs-all → Known issues |
| Relaunch | Pass |

---

## Dirty tree (commit only if human asks)

| Change | Why |
|--------|-----|
| `Cargo.toml` + `scribefloat.rs` | CLI bin → `scribefloat-cli` (case-insensitive macOS was killing the GUI binary in DMGs) |
| `lib.rs` | Sortformer SHA at use-time (startup tray hang ~30–40s) |
| Config / settings / onboarding / MarkdownEditor / clamp helper | **19** / **20** / **21** implementations |
| `.scratch/main-is-god-again/*` | HANDOFF / MAP / tickets **19–21** closed |
| `dist-silicon/` | Build output — **do not commit** |

Re-verify after commit: build → open `.app` → tray appears quickly; `Contents/MacOS/ScribeFloat` stays alive (not CLI help).  
Branch was **ahead 3** of origin when this handoff was written — do not push unless asked.

---

## Session ritual

- Known issues: park only unless human elevates
- Do not commit / push unless asked

---

## Where the tree is

| Area | State |
|------|--------|
| **09** Silicon ship-bar smoke | **Closed** |
| **19** / **20** / **21** | **Closed** (implemented 2026-07-23) |
| **02** Forward working method | Open — next |
| **10** Merge untagged | Blocked by 02 only (09 + 19–21 done) |
| **11** Delete stale branches | After merge |
| **16** / **18** | Closed (IPC B+ / ADR School 1) |
| **17** naming | Known issues — not a merge blocker |

---

## Closed merge-blockers (do not re-litigate)

- Thin-docs, Float UI cut, multi-model delete, bundle-only models
- [Sanitize transcript HTML](./issues/13-sanitize-transcript-html.md)
- [Always delete legacy voice Keychain key](./issues/14-always-delete-legacy-voice-keychain-key.md) — voiceprint **closed**
- [Verify all bundled models before load](./issues/15-verify-all-bundled-models-before-load.md)
- [Least-privilege IPC per window](./issues/16-least-privilege-ipc-per-window.md) — B+
- [Mark and amend ADRs for reality](./issues/18-mark-and-amend-adrs-for-reality.md) — School 1
- Sort / reviews (06 + research)
- [Silicon ship-bar smoke](./issues/09-silicon-ship-bar-smoke.md)
- [Persist onboarding step across quit](./issues/19-persist-onboarding-step-across-quit.md)
- [Onboarding Try Dictate Continue reachable](./issues/20-onboarding-try-dictate-continue-reachable.md)
- [Deleting note text inserts Selection deleted](./issues/21-deleting-note-text-inserts-selection-deleted.md)

---

## Context the next agent must not rediscover

- **Voiceprint never shipped** — local hygiene only; do not re-litigate.
- **Installed `.app`** = honest TCC; persistence works in dev because Notes are files.
- **CLI name** = `scribefloat-cli` (macOS case collision with `ScribeFloat`).
- **SHA-256:** Whisper + Sortformer at use-time; VAD OK at startup.
- **IPC:** new commands → `generate_handler` + `APP_COMMANDS` in `build.rs` + `permissions/sets/` (see `permissions/README.md`).
- **Selection deleted** fixed via CodeMirror `.cm-announced` theme (not TipTap); doc was always clean.
- **Try Dictate Continue:** practice cards use CSS `line-clamp-2` (`maxLines={2}`); history region clipped so Continue stays reachable; no multi-send gate.
- **Onboarding step:** Config `onboarding_step` 1=Welcome 2=Permissions 3=Try Dictate 4=Feature tour.
- **Early Keystroke TCC:** Dictate `CGEventTap` deferred until Input Monitoring preflight is true (fixed 2026-07-23); dialog should no longer appear under Welcome.
- **Push incident:** GitHub once rejected `tests/mic.wav` (~171 MB) in history — rewrite **unpushed** commits only; explain first.

---

## Human hard preference

Do **not** recreate `skills/new-adr` or `skills/new-story`. Use `.scratch/` + `docs/agents/issue-tracker.md`; ADRs as plain files under `docs/adr/`.

---

## How to talk to the human

Plain language. One question at a time when something fails. Prefer common words over ticket-speak.

---

## Suggested skills

| Skill | When |
|-------|------|
| Wayfinder / issue-tracker | Forward method **02**; map gist |
| Commit curator | When human asks — CLI rename + Sortformer fix + **19–21** + scratch |

---

## Stance / do not

Unease + real finding = merge-blocker. “Just get it done” is not a resolution.

- No release tag / website publish this map
- No knowledge / embeddings / retrieval rebuild
- No Upload redesign beyond honesty
- Do not commit unless asked
- Do not re-litigate voiceprint
- Do not block on Known issues (ex-17, Spaces, this-vs-all rename, early TCC, etc.)
