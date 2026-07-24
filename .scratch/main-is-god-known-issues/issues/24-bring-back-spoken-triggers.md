---
title: "Triage: Bring back spoken triggers as Dictate prompt / insert text"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Bring back spoken triggers as Dictate prompt / insert text" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- **Removal commit**: `0f35959f8de27d5144a806d1cb3dbc22c5c7a8c5` ("refactor(output): remove word-replacement engine"), Thu Jul 9 12:13:37 2026 +0100. Parent is `cef8c57` ("feat(build): bundle whisper, VAD, and voiceprint models") — the engine's last-known-good state is `0f35959^`.
- `git show 0f35959 --stat` shows 21 files changed, 178 insertions(+), 1496 deletions(-). Deleted outright:
  - `src-tauri/src/services/output/replacements.rs` (457 lines — the rule engine: `apply_replacements`, `Simple`/`Newline`/`Wrap` rule types, `effective_trigger` prefix-gating, `apply_word_transform`, plus a full test suite)
  - `src/lib/ui/5_views/setting_replace.svelte` (287 lines — Replacements settings tab UI)
  - ~19 lines of replacement-related types from `src/lib/utils/types.ts`
- Also touched (removing call sites / wiring, not deleting the files): `commands/history.rs`, `commands/scribe.rs`, `commands/settings.rs`, `controllers/dictate.rs`, `controllers/history.rs`, `controllers/scribe.rs`, `controllers/settings.rs` (−332 lines, mostly CRUD/IPC for rules), `controllers/transcribe.rs`, `services/analysis.rs`, `services/audio.rs`, `services/note_sidecar.rs`, `services/output/{cleanup,legacy,mod,render,session,wav}.rs`, `services/transcribe_input.rs`.
- Note: the same commit also drops an unrelated `dictate_model_id` config override (dictate always preloads `selected_model_id` now) — that's bundled into this commit but is a separate concern from the replacement engine; don't assume all of `0f35959`'s diff is replacement-related when reverting.
- **Same feature as project memory** (`project_text_replacement.md`, built 2026-05-11, removed 2026-07-09 — ~2 months later): confirmed identical scope — `ReplacementRule`/`ReplacementRuleType`/`ReplacementScope`/`WordTransform` in `types.rs`, `Config.replacement_rules`, `apply_replacements()` engine, Settings → Replacements tab. Not a separate later feature.
- **Original scope was NOT Dictate-only**: `ReplacementScope` had `Both`/per-surface variants and rules applied to transcripts, exports, and history — a general Record/Scribe feature, not scoped to Dictate. `format_dictate_text()` existed as the Dictate-path entry point but shared the same rule engine and rule set as transcribe/scribe.
- **Backup branch not found**: `backup/feature-0.3-embeds-pre-cleanup-20260717` does not exist locally, and `git ls-remote --heads origin` (repo `Lenniott/liscribe_v8`) does not list it either — only `audit/*`, `claude/*`, `feature/*`, `fix/*`, `main`, `refine-audio` are present on origin. The branch may have been deleted, never pushed, or lives in a different remote/fork not currently configured. The old code is still fully recoverable from `0f35959^` regardless.
- **Scope estimate**: Recovering the full engine is a clean revert (`git show 0f35959^:src-tauri/src/services/output/replacements.rs` etc. — the code is intact and tested). But the requested reshape (Dictate-only, insert-prompt behavior, not general Record/Scribe replacements) is a narrower and different feature than what existed: the old engine's `ReplacementScope`, rule CRUD/IPC, and settings-tab UI were all built around multi-surface (transcripts+dictate+history) application. A straight revert would need to be *trimmed down* (drop `ReplacementScope::Both`/transcript-scope handling, drop scribe/history/export call sites, simplify the settings UI to a Dictate-only insert-prompt concept) rather than restored wholesale. This is closer to **revert-then-redesign**: reuse the regex/rule-matching primitives (`apply_replacements`, `replace_whole_word`, `wrap_next_word`, `apply_word_transform` — these are surface-agnostic string transforms) but rebuild the Config schema, settings UI, and wiring for the narrower Dictate-only insert use case rather than un-deleting `replacements.rs` verbatim.
