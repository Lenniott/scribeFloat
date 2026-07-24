---
title: "Triage: Windows file-open / "open with" app path"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Windows file-open / "open with" app path" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- **Windows open helpers already exist in code today** — this is not purely forward-looking. `src-tauri/src/platform/mod.rs:34-55` has a `#[cfg(target_os = "windows")]` implementation of `open_file(path, app)`: if a configured `open_with_app_path` app is set, it runs that executable directly with the file path as an argument (`Command::new(a).arg(path).status()`, lines 38-41); otherwise it falls back to `cmd /c start "" <path>` (lines 50-54) to invoke the OS default handler.
- **Shares validation with macOS path**: the Windows branch is reached through the same `open_file_for_user` → `SettingsController::set_open_with_app_path` (`src-tauri/src/controllers/settings.rs:220-239`) validation used on macOS — requires the configured app path be absolute and exist on disk at set-time. It also inherits whichever confinement fix lands for tickets 19 (extension/folder checks happen before `open_file` is called, in `transcribe.rs`/`settings.rs`, not inside the platform-specific `open_file`), so any hardening applied there covers Windows automatically since it's upstream of the `cfg(target_os)` split.
- **Gaps specific to Windows**: `cmd /c start "" <path>` passes `path` unquoted-adjacent to `start`'s own argument parsing; if `path` contained shell-meta characters it could in principle be misinterpreted, though since `path` at this point has already been canonicalized/validated by the callers (transcribe.rs/settings.rs) and only reachable via the same IPC commands as macOS, the practical risk is the same class as tickets 18/19, not a Windows-only issue. There is no evidence of Windows-specific testing/CI (`platform/mod.rs` and `permissions_impl.rs` have Windows cfg blocks but the project is described as macOS-first "Silicon map" per CLAUDE.md/AGENTS.md), so this code is likely unexercised/unverified on real Windows.
- **Remediation if prioritized**: no distinct fix needed beyond whatever lands for tickets 18/19 (path confinement upstream); optionally quote/validate `path` before interpolating into the `cmd /c start` invocation as defense-in-depth, and add Windows CI coverage.
- **Size estimate**: Small if scoped to "confirm existing code + apply the same upstream path-confinement fix as ticket 19" (no new Windows-specific code needed); Medium if it also means standing up Windows CI/manual verification, which the map already defers ("After main is clean / when Windows care returns").
