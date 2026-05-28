# Branch: `fix/windows-permissions-cmd-flash`

**Status:** Ready to test — merge after Windows + macOS checklist passes.  
**Base:** `main`  
**Commits ahead of main:** 1 (`1456064` — cherry-picked from `claude/windows-permissions-flashing-saHo6`)

Scoped fix only. Model preload/cache and Scribe auto-start work live on other branches.

---

## Summary

Stop the black cmd window that flashes when Settings polls microphone permission on Windows, and hide the Permissions tab there (it had no useful actions).

| File | Change |
|------|--------|
| `src-tauri/src/platform/permissions_impl.rs` | Pass `CREATE_NO_WINDOW` to the `reg query` subprocess. |
| `src/lib/screens/settings.svelte` | Skip permission polling on Windows; hide Permissions tab. |

---

## Problem

Settings polls microphone permission every 10s and on every focus change. On Windows that goes through `reg query`, which inherits a console handle by default and briefly opens a black cmd window on each call.

The Permissions tab on Windows only checked mic status meaningfully; the other cards were hardcoded “Granted” with no action. Windows also auto-prompts for mic on first device access.

---

## Test on Windows

- [ ] Open Settings → leave open 30+ seconds → no cmd window flashes.
- [ ] Switch away and back to Settings repeatedly → no flashes.
- [ ] Permissions tab is **not** visible.
- [ ] General / Models / Replacements / Help tabs still work.
- [ ] Scribe / Dictate still get mic access when recording (OS prompt on first use if needed).

---

## Test on macOS (regression)

- [ ] Permissions tab still visible.
- [ ] Mic / accessibility / etc. cards still poll and update correctly.
- [ ] Opening Settings on focus still refreshes permission status.

---

## Merge checklist

- [ ] Windows flash tests pass on a real Windows build.
- [ ] macOS permissions UI unchanged.
- [ ] `cargo clippy -- -D warnings` and `cargo test -p scribefloat` green (no new tests on this branch).

---

## Related branches

- **`claude/windows-permissions-flashing-saHo6`** — same Windows fix plus model cache/preload, first-run Tiny install, and Scribe auto-start changes (superseded for merge by separate branches).
