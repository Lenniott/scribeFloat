---
title: Mark and amend ADRs for reality
labels: [wayfinder:task, done]
status: closed
assignee: cursor-agent
blocked_by:
  - "06-sort-findings-merge-vs-known.md"
  - "03-adr-reality-audit.md"
parent: MAP.md
---

## Question

Are aspirational ADRs clearly marked “not built yet,” and is ADR-0010 (and similar) amended so they no longer teach fast vs refined Whisper / Settings → Models?

**Done when:** Agents can trust `docs/adr/` status marks; one-model product decision is reflected; history kept (no silent deletes).

Status: ready-for-agent

## Spec (to-spec)

### Problem Statement

Agents treat `docs/adr/` as binding. Several ADRs describe work that is not built (Sources composition, triage, knowledge layer, folder layout, HistoryKind→quick/origin). ADR-0010 still teaches fast vs refined Whisper and Settings → Models, which contradicts the one bundled Small model decision (tickets 08/12). Unmarked aspiration and stale multi-model prose will make the next agent “restore” deleted product surfaces or implement the wrong capture model after merge.

### Solution

**School 1 (agreed):** Keep ADRs that record decisions before or during a wayfinder — including aspirational ones — but stamp each so agents can tell binding vs not-yet-built. Add a light **wayfinder provenance** line (map/effort link + whether that effort is open or complete). Amend ADR-0010 (and sibling multi-model lies) for one bundled Whisper Small. Refresh the ADR index with status (+ provenance where known). Keep history (especially superseded ADR-0011). Do not delete ADRs. Do not implement aspirational systems in this ticket.

Rationale: wayfinder → ADR/tickets/specs → code means ADRs often exist before evidence. School 2 (only binding ADRs) is too harsh for this harness. Prefer one active map at a time; incomplete maps leave ADRs discussable, not silently “law.”

### User Stories

1. As an agent starting a Notes feature, I want ADR-0002 marked aspirational, so that I do not assume a `sources: Vec` model already exists.
2. As an agent touching Float, I want ADR-0004 and ADR-0005 marked aspirational (knowledge/triage not shipped), so that I do not invent triage tables on merge.
3. As an agent creating Notes on disk, I want ADR-0007 marked aspirational, so that I follow the interim `.notes/` layout until a real folder migration effort.
4. As an agent reading capture rules, I want ADR-0010 amended for one bundled Whisper Small, so that I do not rebuild fast/refined Settings → Models.
5. As an agent reading ADR-0010, I want `quick`/`origin` clearly labeled as decided-but-not-fully-built if code still uses `HistoryKind`, so that I do not “finish” a schema migration by accident inside an unrelated PR.
6. As an agent seeing ADR-0011, I want superseded status kept with pointer to ADR-0014, so that voiceprint history explains purge paths without inviting reimplementation.
7. As an agent reading ADR-0003, I want dual-controller unify still described as deferred, and model-tier language removed/updated, so that capture architecture stays honest with one-model (Record/Dictate naming is Known issues, not this map’s merge bar).
8. As a maintainer, I want `docs/adr/README.md` to show status per ADR, so that skimming the index is enough.
9. As a human product conscience, I want no silent ADR deletes, so that decision history survives.
10. As an agent after ticket 17, I want ADR product names (Scribe vs Record) not fighting the UI — amend titles/body where cheap without rewriting every historical “Scribe” mention into fiction.
11. As an agent reading ADR-0013, I want stale “identity = voiceprint” consequence notes corrected or footnoted toward ADR-0014, so that pitch cuts are not tied to biometrics.
12. As a Silicon / merge reviewer, I want a short checklist that every ADR file has an explicit status line, so that “agents can trust marks” is verifiable.
13. As someone opening CONTEXT + ADRs together, I want glossary target model and ADR status to agree on what is built vs future, so that domain docs stop gaslighting implementers.
14. As an agent offered `new-adr` / `new-story` skills, I want those to stay deleted — this ticket only edits `docs/adr/` files, so that retired skills are not recreated.
15. As a later Float effort, I want aspirational ADRs still in place as intent, so that marking is not the same as discarding the decision.
16. As a reader of ADR-0001, I want it to remain binding with optional note about `HistoryRecord` naming debt, so that Note-as-primary stays the rule.
17. As a merge-blocker closer, I want sort items A8 and A10 both addressed by this ticket’s Resolution, so that the ordered blocker list can move to smoke.
18. As an agent amending ADR-0010, I want Upload/Record/Dictate verbs preserved, so that capture surfaces stay the three verbs CONTEXT already uses.

### Implementation Decisions

- **Primary seam:** `docs/adr/` files + index only. Classification source of truth: closed research `research/adr-reality-audit.md` (8 binding / 5 aspirational / 1 superseded).
- **Status marks:** Add a clear Status (or equivalent) on each ADR:
  - **Binding:** 0001, 0003, 0006, 0008, 0009, 0012, 0013, 0014
  - **Aspirational:** 0002, 0004, 0005, 0007, 0010
  - **Superseded:** 0011 (already points at 0014 — keep)
- **ADR-0010 amend (required):** Remove or rewrite fast vs refined Whisper, Settings → Models, and multi-model assignment roles. State one bundled Whisper Small for Record / Dictate / Upload. Keep the still-valid separation of capture verbs vs Note intent; mark `quick`/`origin` vs `HistoryKind` as **not fully implemented** if that remains true after ticket 17’s chosen scope.
- **ADR-0003 amend (light):** Drop “better vs faster” model wording; keep deferred controller unify; align Scribe/Record naming with CONTEXT (Record is the long-form verb).
- **ADR-0013 optional one-liner:** Consequence text must not teach voiceprint identity as current — point at 0014.
- **README:** Status column (or status word per row) matching the audit table.
- **No code changes** required for this ticket. If ticket 17 already renamed user-facing Scribe→Record, ADR prose should not reintroduce Scribe as the current product name for long-form capture.
- **No relocate** of aspirational ADRs out of `docs/adr/` (School 1). Provenance stamps replace exile. Do not recreate deleted doc trees.
- **Do not recreate** `skills/new-adr` or `skills/new-story`.

### Testing Decisions

- “Tests” are checklist / grep style: every `docs/adr/0*.md` has an explicit status; README lists statuses; `rg -i "refined|fast model|Settings → Models|Settings -> Models" docs/adr/` returns no stale product instruction (historical context sentences OK if clearly past).
- No Rust/UI tests required unless a tiny doc test exists — prefer human-readable Resolution checklist.
- Prior art: `research/adr-reality-audit.md` evidence table.

### Out of Scope

- Implementing Sources, triage, knowledge layer, or ADR-0007 folders
- Implementing HistoryKind migration (ticket 17 scope F only; otherwise later)
- Deleting any ADR file
- Website publish / release tag
- Recreating thin-doc trees or ADR authoring skills
- Security IPC (ticket 16)

### Further Notes

- Sort A8 + A10 are this ticket; architecture F6 called ADR-0010 vs one-model a merge-blocker.
- Voiceprint topic closed for this map (ticket 14) — 0011 stays superseded history only.
- After this ticket, agents should prefer CONTEXT + marked ADRs over inventing multi-model UI.

### Open choice (human) — AGREED

- **School 1** with status marks + **wayfinder provenance** stamps (map path / effort slug; open vs complete when known).
- School 2 rejected as too harsh for wayfinder-before-code flow (`AGENTS.md` already: `docs/adr/` = binding **and** aspirational).
- Orphan aspirational ADRs (no live map): still mark aspirational; provenance may say “pre-wayfinder / orphan — revisit before treating as active intent.”

### Done when

1. Every ADR has binding / aspirational / superseded status.
2. Aspirational ADRs have a provenance stamp (wayfinder link or orphan note).
3. No ADR teaches multi-model / Models settings as current product.
4. README index shows status (and provenance summary if cheap).
5. No silent deletes; 0011 remains superseded history.
6. Resolution states School 1 + lists files touched.

## Resolution

**Approach:** School 1 — ADRs stay in `docs/adr/`; each file stamped with **Status** (Binding / Aspirational / Superseded) and **Wayfinder** provenance. No ADRs deleted; aspirational ADRs not relocated.

**Classification applied:** 8 binding · 5 aspirational · 1 superseded (per `research/adr-reality-audit.md`).

**Content amendments:**
- **ADR-0010** — Removed fast/refined tiers, Settings → Models, and multi-model assignment. States one bundled Whisper Small for Record/Dictate/Upload; marks `quick`/`origin` vs `HistoryKind` as not fully implemented.
- **ADR-0003** — Dropped better-vs-faster model wording; aligned Record as long-form verb; kept deferred controller unify.
- **ADR-0013** — Consequence text points at ADR-0014 for speaker identity; voiceprint (0011) cited as superseded history only.
- **ADR-0006** — Removed model from recording-chrome settings popover description.

**Files touched:**
- `docs/adr/0001-note-as-primary-domain-object.md`
- `docs/adr/0002-note-is-a-composition-of-sources.md`
- `docs/adr/0003-scribe-and-dictate-are-capture-profiles.md`
- `docs/adr/0004-triage-is-per-note-not-per-flow.md`
- `docs/adr/0005-knowledge-layer-stored-as-markdown-not-database.md`
- `docs/adr/0006-unified-note-editor-replaces-scribe-and-detail.md`
- `docs/adr/0007-note-folder-structure-and-id-generation.md`
- `docs/adr/0008-codemirror-for-written-source-editor.md`
- `docs/adr/0009-note-lifecycle-immediate-create-autosave-discard-if-empty.md`
- `docs/adr/0010-separate-capture-config-from-note-intent.md`
- `docs/adr/0011-voiceprint-engine-binary-speaker-verification.md`
- `docs/adr/0012-navigation-intent-via-shared-state-flag.md`
- `docs/adr/0013-live-pitch-analysis-and-change-cut-storage.md`
- `docs/adr/0014-anonymous-diarization-replaces-voiceprint-identity.md`
- `docs/adr/README.md`
- `.scratch/main-is-god-again/issues/18-mark-and-amend-adrs-for-reality.md`
- `.scratch/main-is-god-again/MAP.md`

**Verification checklist:**
- [x] Every `docs/adr/0*.md` has explicit Status + Wayfinder lines
- [x] README index lists Status + Provenance columns
- [x] `rg -i "refined|fast model|Settings → Models|Settings -> Models" docs/adr/` — clean (no stale product instruction)
- [x] ADR-0011 remains superseded with pointer to 0014
- [x] No ADR files deleted; no skills recreated
