---
title: "Triage: Heavy native ML / dependency surface"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Resolution

**Now, closed 2026-08-09.** Added [`.github/dependabot.yml`](../../../.github/dependabot.yml): weekly version updates for `npm` (`/`), `cargo` (`/src-tauri`), and `github-actions` (`/`). Minor/patch updates are grouped into one PR per ecosystem so the inbox stays quiet; majors stay ungrouped. Caps: 5 open PRs for npm/cargo, 3 for actions.

Deliberately **not** adding a `cargo audit` / `npm audit` CI job in this ticket — Dependabot covers the “advisory habit” ask without custom workflow code. A blocking audit job can be a future Known issue if we want PR-gate failures.

**Manual check:** After this lands on the **default branch**, open the repo on GitHub → Settings → Code security → Dependabot → confirm Version updates is on. Optionally wait for the weekly run or trigger via the Dependabot “Check for updates” UI. No local app behavior to smoke-test.

## Issue

No dependency or vulnerability scanning exists anywhere in CI — no `cargo audit`, no Dependabot config, no `npm audit` job — despite the app carrying a large native ML dependency surface (Whisper, VAD, Sortformer models and their crates).

## Question

Read the "Heavy native ML / dependency surface" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now** — trivial: adding `.github/dependabot.yml` requires zero custom CI code and gets native GitHub scanning for both Rust and JS deps. Worth doing now, or Later alongside a broader CI pass?

## Findings

- `.github/workflows/` contains only two files: `release.yml` and `reusable-tauri-build.yml`. Both are build/packaging pipelines (macOS arm/intel, Windows) — neither runs any dependency or vulnerability scan.
- No `cargo audit`, `cargo deny`, npm/pnpm `audit`, Dependabot config (`.github/dependabot.yml`), Snyk, Trivy, or OSV scanner step exists anywhere in the repo's CI. (The only `dependabot.yml` found in the tree is inside `node_modules/iconv-lite`, i.e. a third-party package's own repo config — not applicable here.)
- No `Cargo.lock`-based advisory job or `npm audit` job runs on PRs or on a schedule.
- Minimal advisory CI step, if desired later: add a low-cost non-blocking job (e.g. `cargo install cargo-audit && cargo audit` for the Rust side, run on a schedule such as weekly via `on: schedule` plus `on: pull_request` for changed `Cargo.lock`, with failures reported as a warning/annotation rather than blocking the build). Equivalent for JS deps would be `npm audit --audit-level=high` or enabling GitHub's native Dependabot alerts (just add `.github/dependabot.yml` — zero custom CI code required, GitHub runs it natively).
- This is a pure gap confirmation — no scanning exists today. No code changes made.
