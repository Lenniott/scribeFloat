---
title: "Triage: Broad opener / dialog plugin permissions"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Broad opener / dialog plugin permissions" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- **Already fixed** — commit `0129179` "fix: don't trust GitHub API html_url for update release link" (present in `git log`, current HEAD) resolves the specific concern in the original note.
- **Current behavior** (`src-tauri/src/services/update.rs:24-81`): `check_for_update` fetches `RELEASES_URL` (hardcoded `https://api.github.com/repos/Lenniott/scribefloat/releases/latest`) and no longer uses the API's `html_url` field at all — `GithubRelease` (line 11-15) only deserializes `tag_name` and `body`. The URL surfaced to the frontend/opener is built by `release_url_for_tag` (line 75-81), which concatenates a hardcoded trusted base (`REPO_RELEASES_BASE = "https://github.com/Lenniott/scribefloat/releases/tag/"`, line 9) with the tag name after stripping it to `[A-Za-z0-9._-]` only (line 76-79). This means even a fully compromised/MITM'd GitHub API response cannot redirect the opener to an arbitrary URL or scheme — the host and scheme are always the trusted constant, and the tag is sanitized against injection (no `://`, no path traversal, no `javascript:`/custom scheme smuggling).
- **Capabilities/allowlist config** (`src-tauri/capabilities/shell.json`): the `shell` window (main app: Notes, Settings, Record, Upload, and presumably the update-checker UI) has `"opener:default"` and `"dialog:default"` — the default permission sets for both plugins, not scoped to specific URL patterns or file operations. This is broad in the sense that any command reachable from that window's JS can invoke `opener:default`'s allowed commands (typically `open_url`/`open_path`) on whatever URL is passed, but since `release_url_for_tag` is the only current producer of an update-related URL and it's now hardcoded/sanitized, there's no live path from GitHub-controlled data to the opener with an unsafe URL.
- **Remaining residual scope**: `opener:default`/`dialog:default` are still not narrowed by a URL allowlist in `tauri.conf.json`/capabilities — if a future command introduces another externally-sourced URL (e.g. a "docs" link, feedback link, or any other API-provided link) it would inherit the same broad opener permission with no scheme/host allowlist at the capability layer. That's a defense-in-depth gap, not an active vulnerability today.
- **Size estimate**: None needed now for the original concern (already shipped). If the "tighter URL allowlist" hardening in the map is still wanted as defense-in-depth (scoping `opener` capability to `https://github.com/*` via Tauri's URL-pattern permission scoping), that's Small — a capability config change, no Rust code change.
