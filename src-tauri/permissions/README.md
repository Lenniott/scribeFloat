# App IPC permissions (least-privilege)

**New `#[tauri::command]` checklist**

1. Register it in `src/lib.rs` `generate_handler![…]`.
2. Add the snake_case name to `APP_COMMANDS` in `build.rs` (same order is fine; membership matters).
3. Put `allow-<kebab-command>` into the right **permission set** under `sets/`
   (underscores become hyphens — e.g. `scribe_start` → `allow-scribe-start`):
   - `dictate-overlay` — Dictate satellite only
   - `onboarding` — first-run satellite only
   - `main-shell` — main App window (`history` label): Notes / Settings / Record / Upload
4. If the command is high-impact, add it to `SATELLITE_DENY_LIST` in `src/acl_capabilities_test.rs` so satellites cannot gain it by accident.
5. Run `cargo test -p ScribeFloat acl_capabilities` and a quick Dictate + onboarding smoke.

Capabilities live in `../capabilities/` (`dictate.json`, `onboarding.json`, `shell.json`).
Do **not** re-add a single flat capability that grants all app commands to every window.
