# scribefloat — Agent Guide

> Read CONTEXT.md first if you haven't already. Then classify the session below and load only what that mode needs.

---

## Step 1 — Classify the session

Read the user's opening message and pick the mode. If it's ambiguous, ask:
> "Is this an **exploring** session (understanding the system, figuring out what to build), a **building** session (writing code), or a **managing** session (stories, backlog, decisions)?"

| Mode | Signs | Load |
|------|-------|------|
| **Exploring** | "how does X work", "what should we do about Y", "help me understand", writing an exploration or ADR | `CONTEXT.md` + `docs/architecture.md` + `docs/explorations/` |
| **Building** | "implement X", "fix Y", "add Z", writing code | `CONTEXT.md` + relevant docs from the pointer table below |
| **Managing** | stories, backlog grooming, priorities, ADRs | `CONTEXT.md` + `docs/backlog/` + `docs/adr/` |

---

## Step 2 — Load what you need (building sessions)

Pull only the docs relevant to the task. Do not load everything.

```
docs/architecture.md                    ← System diagrams (C4), component maps, module map
docs/engineering/layer-rules.md         ← Adding a feature or IPC command; layer ownership rules
docs/engineering/history-storage.md     ← Note jsonl vs sidecar persistence, autosave
docs/engineering/async-rules.md         ← Controller threading, state machines, Whisper paths
docs/engineering/platform-rules.md      ← macOS threading, audio drain, paste behaviour
docs/engineering/debugging.md           ← Bug investigation table, Whisper debugging
docs/engineering/config-rules.md        ← Adding or changing a Config field
docs/action-flows.md                    ← Step-by-step flows for each workflow
docs/components.md                      ← UI component catalogue
docs/scribe-ui-review.md                ← Before touching Scribe screens or navigation
docs/history-ui-review.md              ← Before touching History screens or components
docs/backlog/active/                    ← Active stories
```

---

## Building code of conduct

- If you change behaviour described in a doc, update that doc in the same session.
- If you add a new layer, service, or platform rule, add it to the relevant `docs/engineering/` file.
- If you add a new UI component or screen, update `docs/components.md`.
- If you change a user-facing flow, update `docs/action-flows.md`.

---

## Skills

**Always write new or updated skills to `skills/`** — never to `.cursor/skills/` or `.claude/commands/` directly. Those directories are managed by `skills/build.sh` and will be overwritten. Run `bash skills/build.sh` after any edit to `skills/` if the PostToolUse hook has not already done so.

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
- If you changed `Config`, verify a file missing the new field still loads (see `docs/engineering/config-rules.md`)
- If you changed `platform/`, verify the other platform compiles: `cargo check --target x86_64-pc-windows-msvc`

---

## Session capture

After any session involving design decisions, architectural choices, or non-obvious implementation choices:

- Check whether anything belongs in `docs/backlog/active/` (`/new-story`) or `docs/adr/` (`/new-adr`)
- If you have gone many exchanges or made decisions without writing either, **stop and ask before wrapping up**
- Any exploration in `docs/explorations/active/` not yet `status: captured` should either be linked to a story/ADR or moved to `docs/explorations/stale/`

---

## Out of scope

Do not build without explicit instruction: Linux, mobile, cloud sync/accounts, pause/resume recording, webhook output, auto-escalation from Dictate to Scribe.
