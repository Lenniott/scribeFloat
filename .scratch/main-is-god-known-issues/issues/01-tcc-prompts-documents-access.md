---
title: "Triage: TCC prompts fire too early (Documents access remainder)"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "TCC prompts fire too early (Documents access remainder)" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- Default save folder is `~/Documents/transcripts_scribefloat` on macOS/Linux: `src-tauri/src/types.rs:189-194` (`default_save_folder`), Windows equivalent `types.rs:182-187`.
- Reproduces as written: on every app start (including first run / before onboarding's Permissions step even shows), `src-tauri/src/lib.rs:781-815` reads `config.save_folder` and spawns a background task that touches it unconditionally:
  - `history_bg.compact(&save_folder_bg)` → `src-tauri/src/services/history.rs:276` → `ensure_loaded` (`history.rs:45-56`) calls `path.exists()` on `{save_folder}/history.jsonl`, which is under `~/Documents/…` and triggers the macOS Documents-folder TCC prompt even on a plain existence check.
  - `output_bg.scan_incomplete_scribe_sessions(&save_folder_bg)` → `src-tauri/src/services/output/legacy.rs:114-124` calls `root.is_dir()` then `std::fs::read_dir(&root)` on the same default folder.
  - `output_bg.scan_and_salvage_dictate_temp_wavs(...)` (same block, `lib.rs:810-814`) also touches `save_folder_bg`.
- This background task runs regardless of `is_first_run` and regardless of whether the user has been through onboarding's explicit folder setup — there is currently no "explicit folder setup" step in onboarding at all (`src/lib/ui/5_views/onboarding.svelte` steps are Welcome → Permissions → DictatePractice → FeatureTour; no save-folder chooser). So the Documents TCC prompt can fire at process launch, before the Permissions step is even shown.
- A fix would concretely touch: `src-tauri/src/lib.rs:781-815` (defer/gate the background compaction+scan spawn), and possibly `types.rs` default folder choice or an onboarding step to have the user confirm/pick the save folder before any fs touch. Note history.rs's `compact()` has an early return before `create_dir_all` when the store doesn't exist and there are no live records (`history.rs:282-283`), but the earlier `ensure_loaded` `path.exists()` check (line 54) still touches the path before that guard is reached.
- Size estimate: small–medium. The minimal fix (skip/delay the startup background scan until after onboarding completes or the save folder is confirmed) is small; making the save folder itself configurable during onboarding before first touch would be medium.
