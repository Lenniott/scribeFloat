# scribefloat — Agent Guide

> Read `CONTEXT.md` first if you haven't already. Then classify the session below and load only what that mode needs.

---

## Step 1 — Classify the session

Read the user's opening message and pick the mode. If it's ambiguous, ask:
> "Is this an **exploring** session (understanding the system, figuring out what to build), a **building** session (writing code), or a **managing** session (stories, backlog, decisions)?"

| Mode | Signs | Load |
|------|-------|------|
| **Exploring** | "how does X work", "what should we do about Y", "help me understand", writing an ADR | `CONTEXT.md` + relevant `docs/adr/` + code under the area in question |
| **Building** | "implement X", "fix Y", "add Z", writing code | `CONTEXT.md` + relevant `docs/adr/` + code; design skill before Tailwind |
| **Managing** | priorities, Known issues, ADRs, wayfinder maps | `CONTEXT.md` + `docs/adr/` + `docs/agents/` + `.scratch/<effort>/` |

There is no separate architecture / action-flows / engineering / backlog / explorations tree. Prefer code + ADRs over inventing replacement essays.

---

## Canonical docs (keep-set)

| Path | Role |
|------|------|
| `CONTEXT.md` | Domain glossary — read first |
| `PRIVACY.md` | Privacy claims agents must not contradict |
| `docs/adr/` | Binding and aspirational decisions |
| `docs/agents/` | Tracker, triage labels, domain-doc consumption, working method |
| `AGENTS.md` | This file — session index, build, skills |
| `src-tauri/permissions/` | Per-window IPC allowlists — see `permissions/README.md` when adding `#[tauri::command]` |

Working memory for an effort lives under `.scratch/<effort-slug>/` (maps, tickets, Known issues). See `docs/agents/issue-tracker.md`.

---

## Skills

**Always write new or updated skills to `skills/`** — never to `.cursor/skills/` or `.claude/commands/` directly. Those directories are managed by `skills/build.sh` and will be overwritten. Run `bash skills/build.sh` after any edit to `skills/`.

**Frontend:** before writing any Tailwind classes, query the design skill:
```bash
python3 skills/design-skill/query.py ds toc        # token/component index
python3 skills/design-skill/query.py ux toc        # UX playbook index
python3 skills/design-skill/query.py search "X"    # search both
```

---

## Build and test

```bash
cargo tauri dev                    # Start dev build
cargo test -p ScribeFloat          # Unit tests (no hardware required)
cargo clippy -- -D warnings        # Must pass before committing
cargo check                        # Fast compile check
```

### Bundled models

Release builds bundle the Whisper Small, Silero VAD, and voiceprint ONNX models
(`tauri.conf.json` → `bundle.resources`). **Run `scripts/fetch-bundled-models.sh`
before `cargo tauri build`** to download them into `src-tauri/bundled-models/`
(gitignored). The Tauri build script requires those three paths to exist even for
`cargo check` — on a fresh clone either run the fetch script or create 0-byte
placeholders (`touch src-tauri/bundled-models/<name>`). Startup seeding skips
empty files, so dev builds with placeholders simply run without the models.

### Hardware-gated tests

Some tests are marked `#[ignore]` because they require a real mic or macOS
loopback device. They are **skipped in CI and virtual environments** — do not
run them there. On a developer machine with hardware:

```bash
cargo test -p ScribeFloat -- --ignored          # hardware-gated tests only
cargo test -p ScribeFloat -- --include-ignored  # everything
```

Currently gated: `mic_session_*` (real mic, any OS), `loopback_session_*`
(macOS speaker capture). Do **not** add `#[ignore]` to tests that can run
without hardware — use it only when the test genuinely requires a device.

---

## Before committing

- `cargo clippy -- -D warnings` passes
- `cargo test -p ScribeFloat` passes
- If you changed a `#[tauri::command]` signature, verify the JS caller uses matching camelCase argument names
- If you changed `Config`, verify a file missing the new field still loads (serde defaults / `#[serde(default)]`)
- If you changed `platform/`, verify the other platform compiles: `cargo check --target x86_64-pc-windows-msvc`

---

## Session capture

After any session involving design decisions, architectural choices, or non-obvious implementation choices:

- Binding decisions → `docs/adr/` (new file + index line)
- Effort work / niggles → `.scratch/<effort>/` (tickets or `KNOWN-ISSUES.md`)
- If you have gone many exchanges or made decisions without writing either, **stop and ask before wrapping up**

Full forward process (session classification, thin-doc rules, Known issues path, when to ADR, merge-blocker vs park, session bridge, public tag as separate effort): `docs/agents/working-method.md`.

---

## Out of scope

Do not build without explicit instruction: Linux, mobile, cloud sync/accounts, pause/resume recording, webhook output, auto-escalation from Dictate to Scribe.

---

## Agent skills

### Issue tracker

Local markdown under `.scratch/<feature>/` (no GitHub Issues; PRs are not a triage surface). See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.

### Working method

Session classification, thin-doc rules, Known issues path, when to write an ADR,
merge-blocker vs park, the HANDOFF.md session bridge, public tag as a separate
effort. See `docs/agents/working-method.md`.
