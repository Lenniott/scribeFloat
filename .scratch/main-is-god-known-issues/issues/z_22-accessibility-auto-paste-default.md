---
title: "Triage: Accessibility + auto-paste on by default"
labels: [wayfinder:grilling]
status: closed
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

New installs default `dictate_auto_paste` to `true`, so Dictate auto-pastes into whatever app is focused (requiring macOS Accessibility permission) without an explicit opt-in. **Correction (2026-07-29): the original Findings below wrongly claimed a Settings toggle already exists — it doesn't.** The backend get/set commands (`settings_get_dictate_auto_paste`/`settings_set_dictate_auto_paste`) exist and are wired in `lib.rs`, but nothing in the frontend calls them; there is no row for this anywhere in `src/lib/ui/5_views/setting_*.svelte`. Users currently have no way to turn it off.

## Question

Read the "Accessibility + auto-paste on by default" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Product decision made**: add a toggle to Advanced settings (`setting_advanced.svelte`), leaving the default as `true` — since the config field and backend commands already exist, this is just wiring a new `SettingsRow`/`ToggleSwitch` following the exact pattern already used for "Keep audio after transcription" in that file (`setting_advanced.svelte:117-125`), backed by `settings_get_dictate_auto_paste`/`settings_set_dictate_auto_paste`.

## Resolution

**Done (2026-07-29).** Added an "Auto-paste after Dictate" toggle to `setting_advanced.svelte`'s new "Dictate" section, backed by the existing `settings_get_dictate_auto_paste`/`settings_set_dictate_auto_paste` commands. Default stays `true` per product decision — this only fixes the missing UI, not the default. Test-first: added `loads and persists the dictate auto-paste toggle` to `setting_advanced.test.ts`, confirmed it failed before the component change, then implemented. Full `setting_advanced.test.ts` suite (5 tests) and all of `src/lib/ui/5_views/` (27 tests) pass.

## Findings (superseded — kept for record)

- **Default is set in the config struct's `Default` impl**: `src-tauri/src/types.rs:99` declares `pub dictate_auto_paste: bool` on the settings/config struct, and `src-tauri/src/types.rs:137` sets `dictate_auto_paste: true` as the default value used when config is first created (new install / no persisted config yet).
- **Runtime get/set**: `SettingsController::get_dictate_auto_paste` / `set_dictate_auto_paste` (`src-tauri/src/controllers/settings.rs:285-293`) are thin read/persist wrappers around `cfg.dictate_auto_paste`, exposed as Tauri commands and registered in `lib.rs`, but **not called from any frontend view** — confirmed by grepping `dictate_auto_paste`/`auto_paste` across `src/`, which returns zero hits outside the generated command names. The claim that a toggle "already exists in Settings" was wrong.
- **Behavior when on**: paste happens via `OutputService::paste_text` (`src-tauri/src/services/output/mod.rs:191-193`), which shells out to `crate::platform::paste_impl::paste_text()` — a simulated Cmd/Ctrl+V into the currently focused application, requiring Accessibility permission on macOS.
- **What a fix touches**: `src/lib/ui/5_views/setting_advanced.svelte` only — add `dictateAutoPaste` state, a `refresh()` fetch, a `setDictateAutoPaste` handler, and a new `SettingsRow`/`ToggleSwitch` pair, mirroring the existing "Keep audio after transcription" row exactly (lines 49-60, 117-125). No backend change needed.
- **Size estimate**: Small — one file, copy-paste of an existing pattern in the same component, no backend work.
