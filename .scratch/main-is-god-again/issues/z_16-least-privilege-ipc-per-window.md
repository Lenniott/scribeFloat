---
title: Least-privilege IPC per window
labels: [wayfinder:task, ready-for-agent]
status: closed
assignee: cursor-agent
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
parent: MAP.md
---

## Question

Can Dictate / onboarding / main shell each call only the Tauri commands they need — instead of one flat “every window may call everything” capability list?

**Done when:** Satellite windows cannot invoke unrelated high-impact commands; capability split is documented enough for the next agent.

Status: closed

## Spec (to-spec)

### Problem Statement

Today every webview that shares the app’s capability can call every registered Tauri command. Dictate and onboarding are small satellite windows; the main shell (window label `history`) holds Notes, Settings, Record, Upload. If JS in a satellite is compromised (or a bug calls the wrong command), that window can still reach high-impact actions such as deleting Notes, changing open-with paths, starting Upload/Record, or rewriting settings. Security review S6 called this out; ticket 13 closed the transcript HTML XSS amplifier, but the flat ACL remains.

### Solution

Split capabilities so each window only gets the commands and plugin permissions its real UI needs. Satellite windows fail closed on unrelated invokes. The main shell keeps the broad command set required for Notes / Settings / Record / Upload. A short matrix (window → allowed command groups) lives with the ticket Resolution or a small comment near the capability files so the next agent does not re-flatten the ACL.

### User Stories

1. As a Dictate user, I want the Dictate overlay to only drive Dictate (cancel / dismiss / state), so that a bug or compromise in that window cannot delete Notes or change Settings.
2. As a first-run user in onboarding, I want onboarding to only complete setup, request permissions, and show the main window, so that the onboarding webview is not a back door into Record/Upload/history deletes.
3. As a Notes user in the main shell, I want Record, Upload, history, and Settings commands to keep working as they do today, so that least-privilege does not break the primary app.
4. As a Settings user, I want open-with path and output-folder changes to remain available only from the main shell, so that satellites cannot retarget file openers.
5. As an Upload user, I want Upload commands callable only from the main shell, so that Dictate/onboarding cannot start bulk transcription.
6. As a Record user, I want Record (`scribe_*` today) commands callable only from the main shell, so that satellites cannot start or stop long-form capture.
7. As someone whose Dictate webview is compromised, I want `history_delete` denied, so that Notes cannot be wiped from that surface.
8. As someone whose onboarding webview is compromised, I want `transcribe_start` and `settings_set_open_with_app_path` denied, so that high-impact paths stay out of first-run UI.
9. As a Dictate user who pastes text, I want clipboard write to remain available to Dictate if the product still needs it, so that paste/clipboard behaviour does not regress without an explicit decision.
10. As an onboarding user granting mic access, I want permission request/status commands available to onboarding, so that the permissions step still works.
11. As a Silicon tester, I want a clear “satellite invoke should fail” check I can reason about, so that ship-bar smoke can trust the ACL story.
12. As a security reviewer, I want S6 closed with evidence: per-window capabilities + no default “all commands for all windows.”
13. As a maintainer, I want the allowlists derived from what each window’s frontend actually invokes (plus tray/hotkey paths that are not webview invokes), so that we do not invent fake APIs or over-allow “just in case.”
14. As an agent implementing ticket 17 later, I want command renames (if any) to update the same capability allowlists, so that rename work does not silently re-open flat ACL.
15. As a Windows user later, I want the same capability identifiers and window labels, so that least-privilege is not macOS-only.
16. As a user opening the main window from onboarding, I want `settings_show_window` (or equivalent) still allowed from onboarding, so that first-run completion is not broken.
17. As a user practicing Dictate during onboarding, I want only the minimal Dictate-related settings toggles onboarding already uses, so that practice works without granting full Settings write.
18. As a reader of PRIVACY / security notes, I want the capability split described in plain language in the Resolution, so that auditors see windows are not equivalent.

### Implementation Decisions

- **Primary seam:** Tauri 2 capabilities + app-command permissions. Today one `default` capability lists windows `dictate`, `history`, `onboarding` and only names plugin permissions; registered app commands are effectively available to all of those windows. Change the ACL so commands are not globally allowed; grant per window (or per capability file).
- **Window map (current labels):**
  - `dictate` — Dictate overlay
  - `onboarding` — first-run
  - `history` — main App shell (Notes / Settings / Record / Upload live here)
- **Recommended aggression (default for this spec):** **(B) Strict inventory** — For each satellite, allow only commands that window’s Svelte code actually invokes today (plus any event/plugin needs those views already use). Main/`history` gets the remaining app commands needed by the shell. Deny by omission.
- **Aggression alternatives (human may pick):**
  - **(A) Coarse groups** — Three capabilities with broad groups (`dictate-*`, `onboarding-*`, `shell-*`) that may over-allow within a group but still block cross-window high-impact calls.
  - **(B) Strict inventory** — Preferred; smallest satellite surfaces.
  - **(C) Strict + deny-list tests** — Same as B, plus automated tests that satellites cannot invoke a fixed deny list (`history_delete`, `transcribe_start`, `scribe_start`, `settings_set_open_with_app_path`, …).
- **Inventory method:** Grep/frontend audit of `invoke(` under Dictate and onboarding views/sections; treat main shell as the default broad surface. Tray / global shortcuts that call Rust directly are out of the webview ACL (already native).
- **Plugin permissions:** Split as needed (e.g. clipboard for Dictate; dialog/opener for main). Do not leave `opener:default` / `dialog:default` on satellites unless a satellite truly uses them. Tightening opener URL allowlists globally remains Known issues (S9), not this ticket.
- **Documentation:** Resolution includes a short matrix: window label → capability id → command groups / notable denials. Enough for the next agent; no new essay tree.
- **No runtime behaviour change** for honest UI paths on the main shell.
- **Command renames** (Scribe→Record) are ticket 17; if 16 lands first, use current `scribe_*` / `transcribe_*` names in allowlists and expect 17 to update them.

### Testing Decisions

- Good tests assert **external behaviour**: from a given window label / capability set, a denied command is rejected by the ACL; an allowed command remains invokable. Prefer not to assert internal file layout of every permission TOML beyond what capability wiring requires.
- Prefer a small deny-list integration or unit-level check if the Tauri ACL is hard to unit-test in-process; at minimum, document a manual Silicon check: open Dictate, attempt a denied invoke from the webview console or a temporary test hook — should fail.
- Prior art: security-review evidence paths; no existing per-window ACL tests found — this ticket may introduce the first.
- `cargo test -p ScribeFloat` and `cargo clippy -- -D warnings` must pass; app must still boot Dictate, onboarding, and main flows.
- After implement: run security-review skill briefly against S6 for evidence closeout (HANDOFF).

### Out of Scope

- Dialog-only Upload path tokens (S7)
- Confining `transcribe_open_output` to save folder (S8)
- Global opener URL allowlist redesign (S9)
- Windows `open_file` hardening (S10)
- Changing Dictate auto-paste defaults (S11)
- Merging Record/Dictate controllers (ticket 17 / ADR-0003)
- ADR status marks (ticket 18)
- CSP redesign beyond what ACL work requires

### Further Notes

- Evidence: `research/security-review.md` §6; sort ticket 06 elevated S6 to merge-blocker.
- Tauri default: registered commands are allowed for all windows until the app restricts them via capabilities / AppManifest command permissions — that is the bug class, not a missing plugin flag.
- Ticket 13 reduced XSS blast radius; this ticket reduces what a compromised satellite can call even if XSS returns.

### Open choice (human) — AGREED

- **Aggression: B+** (2026-07-19 human).
- Inventory allowlists + **named permission sets** + maintainer note (“new command → which set?”) + **cheap static deny-list** test (capability files for `dictate`/`onboarding` must not grant high-impact commands). Full runtime invoke-from-window harness not required.
- Window labels confirmed: `history` = main shell.

### Done when

1. Dictate and onboarding cannot invoke unrelated high-impact commands (deny-by-omission).
2. Main shell product paths still work.
3. Capability matrix + “where do new commands go?” convention documented in Resolution (and a nearby maintainer note).
4. Prefer B+ static deny-list guard if cheap; tests/clippy clean; S6 evidence updated or pointed from Resolution.

## Resolution

**Aggression: B+** (2026-07-19).

| Cut | Result |
|---|---|
| AppManifest | `build.rs` lists all 86 commands → ACL on (no global allow-all) |
| Capabilities | Split: `dictate.json`, `onboarding.json`, `shell.json` (window `history`) — removed flat `default.json` |
| Permission sets | `dictate-overlay`, `onboarding`, `main-shell` under `permissions/sets/` |
| Satellite inventory | Dictate: cancel/dismiss + theme; Onboarding: permissions + complete + show main + dictate auto-enter get/set + theme |
| Plugins | opener/dialog/clipboard only on shell |
| Maintainer note | `permissions/README.md` + `AGENTS.md` pointer |
| Static tests | `acl_capabilities_test.rs` — deny-list, capability wiring, handler↔APP_COMMANDS sync |

**Verify:** `cargo test -p ScribeFloat` → 349 passed, 5 ignored; `cargo clippy -p ScribeFloat -- -D warnings` clean.

**S6:** Closed with evidence — satellites cannot be granted deny-list commands via capability files; runtime ACL enforced by Tauri once AppManifest lists commands.

