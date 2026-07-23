# Architecture and single-model review

**Ticket:** [Architecture and single-model review](../issues/05-architecture-single-model-review.md)  
**Spine:** `feature/0.3/embeds` → merge into `main` (untagged)  
**Hard requirement:** Product is **one model**. Merge needs UI/product truth **and** deletion of dead multi-model download/chooser paths (deletion itself is ticket *Delete dead multi-model paths*; this doc inventories them).

---

## Rubric

Score each area against merge confidence for *Main is God again*. Failures that leave multi-model chooser/download as a reachable or marketed path are **merge-blockers**. Structural debt that does not re-offer multi-model choice is **Known issues** unless it blocks understanding the spine.

### 1. Seams / testability

| Expectation | Pass signal |
|---|---|
| Controllers own workflows; services are unit-testable without UI | Capture/transcribe paths callable with faked `ModelService` / temp dirs |
| IPC surface matches product | No dead chooser commands; remaining commands have real callers |
| Config migrate-friendly | Missing/obsolete fields load; retired keys ignored |
| State machines are named and observable | Record/Dictate/Upload emit coherent stage events |

### 2. Capture pipeline (Record / Dictate / Upload)

| Expectation | Pass signal |
|---|---|
| All three produce a **Note** (`HistoryRecord`) | Shared history write path; `quick` / `origin` distinguish intake |
| Same Whisper engine, different capture config | Audio durability, hotkey vs in-app, paste vs save — **not** a second model tier |
| Failure modes honest | Missing bundled model → clear error that does **not** send users to a deleted Models settings tab |

### 3. Notes / speakers

| Expectation | Pass signal |
|---|---|
| Note is the unit of storage and recall | JSONL + sidecars; titles/tags editable |
| Speakers are anonymous labels with rename | Relabel cascades; names vocabulary in settings |
| No identity/cloud dependency for core path | Local diarization / names only |

### 4. Single-model truth

| Expectation | Pass signal |
|---|---|
| One Whisper ASR model in product | Bundled Small (`small-en-q5` / `ggml-small.en-q5_1.bin`) |
| No user-facing download/chooser | No Settings → Models, no onboarding model step, no Upload model picker |
| No reachable backend chooser API | No `model_list` / `model_download` / `model_select` / `model_remove` IPC |
| Docs and marketing match code | README / PRIVACY / site / domain glossary do not sell multi-size download or fast vs refined Whisper tiers |
| Dead selection machinery removed or clearly inert | Prefer delete over “catalog of one” + `selected_model_id` dual path |

---

## Findings

### F1 — App UI no longer offers multi-model download/choose (good)

**Evidence:** Commit `cef8c57` removed `setting_models.svelte`, `ModelDownloadStep.svelte`, `modelDownload.svelte.ts`, and IPC `model_list` / `model_download` / `model_select` / `model_remove` / `model_vad_download` / `model_vad_remove`. Current settings tabs are `general | advanced | voice | permissions | help` (`src/lib/ui/4_sections/settingsTypes.ts`). Onboarding is Welcome → Permissions → DictatePractice → FeatureTour (`src/lib/ui/5_views/onboarding.svelte`). Upload always passes `modelId: null` (`src/lib/ui/5_views/transcribe.svelte`).

**Verdict:** Not a merge-blocker for *in-app* chooser UI (already gone). Remaining work is dead backend + product copy (below).

### F2 — Product / docs still market multi-model download (merge-blocker)

**Evidence:**

| Surface | Lie |
|---|---|
| `README.md` | “Choose from several Whisper model sizes. Downloaded once from Hugging Face…” |
| `PRIVACY.md` | User-initiated Hugging Face downloads via Settings → Models; `dictate_model_id` still listed |
| `site_content.json` | Setup step 02: “Open Settings → Models and click Download…” |
| `CONTEXT.md` | Record = “Refined transcription model”; Dictate = “Fast transcription model”; capture config still mentions “model quality tier” |
| `docs/adr/0010-…` | Explicitly keeps fast/refined model distinction; references `setting_models.svelte` |

**Verdict:** **Merge-blocker** — map requires UI *and* product truth for one model. (Coordinate with thin-docs / ADR audit tickets; do not leave public copy selling a deleted Settings tab.)

### F3 — Dead multi-model selection machinery still in runtime (merge-blocker for ticket 08)

**Evidence:** Even with a one-entry catalog, the app still behaves like a chooser:

- `Config.selected_model_id` + `Config.scribe_model_path` (`src-tauri/src/types.rs`) — comments still say “selected by user in model setup” / “NO_MODEL if none”.
- `ModelController::select_model` (`src-tauri/src/controllers/model.rs`) — no IPC; only startup auto-select in `lib.rs`.
- Startup migration auto-selects `DEFAULT_MODEL_ID` when selection missing/obsolete (`src-tauri/src/lib.rs` ~634–648).
- Divergent resolvers: `resolve_model_path` in `scribe.rs` (path then id), `resolve_dictate_model_path` in `dictate.rs` (id then path), `resolve_model_path` in `transcribe.rs` (explicit id → selected id → default).
- `transcribe_start(..., model_id: Option<String>)` still accepts a chooser override (`src-tauri/src/commands/transcribe.rs`); UI always sends `null`.
- `MODEL_CATALOG` + `model_path_for_id` / `model_downloaded` / `catalog_item` (`src-tauri/src/services/model.rs`) — multi-id API for a single bundled file.
- User-visible error still points at deleted UI: Dictate `"No Whisper model available. Download one in Settings → Models."` (`dictate.rs` ~808). Integrity failure text: `"Re-download from Settings."` (`model.rs` ~568).

**Verdict:** **Merge-blocker** (deletion set for *Delete dead multi-model paths*). Keep bundled seed + integrity checks; collapse resolution to the single bundled path.

### F4 — Capture pipeline seams are coherent enough; dual controllers are debt (Known issues)

**Evidence:** Record (`ScribeController`), Dictate (`DictateController`), and Upload (`TranscribeController`) all go through `ModelService` transcription and write `HistoryRecord` with `quick` / `origin`. `CONTEXT.md` already names the two-controller split as an evolution artefact. Naming still says “Scribe” in code/commands while domain says “Record”.

**Verdict:** **Known issues** — does not re-offer multi-model choice; unify/rename is post-merge polish unless it blocks Silicon smoke.

### F5 — Notes / speakers model is directionally right (Known issues for gaps)

**Evidence:** Notes use `quick` + `origin`; history IPC includes `note_relabel_speaker`; `SpeakerNameService` + Voices settings (`setting_voice.svelte`). Map domain (anonymous speakers, rename cascade) is present in spine code. `HistoryRecord.model` stores a label string for provenance (e.g. `"small.en-q5_1"`) — metadata, not a chooser.

**Verdict:** **Known issues** for incomplete Float/layer UI teases and any rename-edge cases; not a single-model merge-blocker.

### F6 — ADR-0010 and glossary contradict one-model decision (merge-blocker / ADR ticket)

**Evidence:** ADR-0010 status Accepted; preserves fast/refined tiers and `setting_models` label work. Glossary still teaches two Whisper quality defaults per capture method.

**Verdict:** **Merge-blocker** for doc truth (likely folded into *ADR reality audit* + thin docs). Code already uses one bundled model for all capture paths when present.

### F7 — Stale references to deleted UI in skills/plans/audits (Known issues)

**Evidence:** `skills/ui-taxonomy/SKILL.md` still lists `ModelDownloadStep`; `.cursor/plans/frontend_flow_test_plan_*.plan.md`, `docs/audits/*`, backlog `0043-…` still mention `model_list` / `setting_models` / `ModelDownloadStep`.

**Verdict:** **Known issues** — not user-facing; clean when touching those docs. Prefer not to block merge solely on audit/plan rot.

---

## Inventory — multi-model UI + dead paths (recommended deletion / rewrite)

> Do **not** delete in this ticket. Feed *Delete dead multi-model paths* and docs tickets.

### A. Already deleted (verify stay gone)

| Path | What it was | Status |
|---|---|---|
| `src/lib/ui/5_views/setting_models.svelte` | Settings → Models download/select/remove | Deleted `cef8c57` |
| `src/lib/ui/4_sections/onboarding/ModelDownloadStep.svelte` | Onboarding model download step | Deleted `cef8c57` |
| `src/lib/stores/modelDownload.svelte.ts` (+ `.test.ts`) | Download progress / list store | Deleted `cef8c57` |
| IPC: `model_list`, `model_download`, `model_select`, `model_remove`, `model_vad_download`, `model_vad_remove` | Chooser + download commands | Removed from `commands/model.rs` / `generate_handler` |

### B. Dead / multi-model-shaped code — delete or collapse (merge-blocker set)

| Path | What to remove / simplify | Notes |
|---|---|---|
| `src-tauri/src/controllers/model.rs` → `select_model` (+ tests for multi-id select) | Selection API | Startup can set default path without catalog-id chooser |
| `src-tauri/src/types.rs` → `Config.selected_model_id`, `Config.scribe_model_path` | Dual selection fields | Prefer single implied bundled path; keep serde ignore for old keys if needed |
| `src-tauri/src/lib.rs` startup `select_model` block | Catalog migration chooser | Replace with “ensure bundled Small present” |
| `src-tauri/src/services/model.rs` → `MODEL_CATALOG`, `ModelCatalogItem`, `model_path_for_id`, `model_downloaded`, `catalog_item`, catalog-oriented comments | Multi-id catalog façade | Keep filename + SHA for the one bundled Whisper file |
| `src-tauri/src/controllers/scribe.rs` → `resolve_model_path` / `preload_path_for_config` (+ tests using `tiny-en-q5` / custom paths) | Multi-fallback resolver | Resolve only bundled default (plus integrity) |
| `src-tauri/src/controllers/dictate.rs` → `resolve_dictate_model_path` / preload helper (+ tests) | Same | Fix error string (no Settings → Models) |
| `src-tauri/src/controllers/transcribe.rs` → `resolve_model_path(..., explicit_model_id)` + `TranscribeRequest.model_id` | Per-job model override | |
| `src-tauri/src/commands/transcribe.rs` → `model_id` arg | IPC chooser remnant | |
| `src/lib/ui/5_views/transcribe.svelte` → `modelId: null` | Dead arg | Drop once command drops param |
| `src-tauri/src/types.rs` → `ModelDownloadEvent` comment + Whisper-catalog wording | Event still used for **VAD** auto-download | Keep type for VAD; rewrite comment so it is not a Whisper chooser channel |
| Error / integrity copy in `dictate.rs`, `model.rs` | “Settings → Models” / “Re-download from Settings” | Point at reinstall / bundled seed failure |

### C. Product / domain copy — rewrite for one model (merge-blocker)

| Path | Action |
|---|---|
| `README.md` | Remove “several Whisper model sizes” / HF download feature row; describe bundled model |
| `PRIVACY.md` | Align network story with bundled Whisper + optional/auto VAD; drop Settings → Models |
| `site_content.json` | Replace setup step 02 (download a model) |
| `CONTEXT.md` | Drop fast vs refined Whisper defaults; one model for Record/Dictate/Upload |
| `docs/adr/0010-separate-capture-config-from-note-intent.md` | Supersede or amend: quality tier ≠ multiple Whisper downloads (ADR audit) |

### D. Stale references — Known issues cleanup

| Path | Action |
|---|---|
| `skills/ui-taxonomy/SKILL.md` | Drop `ModelDownloadStep` |
| `.cursor/plans/frontend_flow_test_plan_c9f82844.plan.md` | Drop model download flow cases |
| `docs/audits/typography-audit.md`, `docs/audits/color-audit.md` | Drop `setting_models` / `ModelDownloadStep` rows |
| `docs/backlog/active/0043-component-behaviour-tests.md` | Drop deleted test names |
| `docs/explorations/active/2026-07-05-architecture-deepening-candidates.md` | Historical; move/stale or annotate |

### E. Keep (not multi-model chooser)

| Path | Why keep |
|---|---|
| `scripts/fetch-bundled-models.sh`, `src-tauri/bundled-models/`, `tauri.conf.json` resources | Single-model shipping |
| Startup seed copy of Small + VAD + Sortformer (`lib.rs`) | Required for one-model product |
| `download_vad_model` (startup, not user chooser) | Small auto-fetch; not Whisper size picker |
| `HistoryRecord.model` provenance string | Note metadata |
| `model_vad_status` IPC | Status only |
| Embedding model ids in `services/context_search.rs` | Out of map scope (retrieval); not Whisper ASR chooser |

---

## Rubric scorecard (spine today)

| Area | Result | Blocking? |
|---|---|---|
| Seams / testability | Controllers/services testable; leftover chooser APIs confuse the seam | Partial — cleanup in F3 |
| Capture pipeline | Same engine; dual controllers + Scribe naming | Known issues (F4) |
| Notes / speakers | Solid core; Float teases elsewhere | Known issues (F5) |
| Single-model truth | In-app chooser gone; docs + dead selection paths remain | **Fail — merge-blockers F2, F3, F6** |

---

## Suggested sort (input to *Sort findings into merge-blockers vs Known issues*)

**Merge-blockers**

1. Delete / collapse inventory **B** (dead selection + Upload `model_id` + Settings→Models error copy).
2. Rewrite inventory **C** (README / PRIVACY / site / CONTEXT; ADR-0010 via ADR ticket).
3. Confirm no regression of deleted UI in **A**.

**Known issues**

- Dual `ScribeController` / `DictateController` and Record rename lag (F4).
- Stale skills/plans/audits (F7 / inventory **D**).
- Speaker/Float polish beyond rename cascade (F5).
