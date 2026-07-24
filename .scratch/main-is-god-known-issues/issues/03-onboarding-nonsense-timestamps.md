---
title: "Triage: Onboarding Try Dictate shows nonsense timestamps"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Onboarding Try Dictate shows nonsense timestamps" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- Root cause found and confirmed exactly matches the reported symptom (`495740:07:43`).
- `DictatePracticeStep.svelte:39` stores `recordedAtMs: Date.now()` — a correct absolute epoch-ms value — on each practice note.
- `InlineNote.svelte:57` renders it via `<TimestampLabel at={note.recordedAtMs} />` when no explicit `timestampLabel` string is passed (practice cards use this default path since `DictatePracticeStep`/`NoteCard` usage doesn't pass `timestampLabel`).
- `src/lib/ui/1_primitives/display/Timestamp.svelte` `format()` (`:9-14`) treats `at` as an **elapsed duration since zero**, not a wall-clock epoch: it does `Math.floor(ms/1000)` then breaks into hours/minutes/seconds with no modulo on hours, so it renders `HH:MM:SS` where HH can be arbitrarily large. Feeding it `Date.now()` (~1.75×10^12 ms) yields `hours = floor(1.75e12/1000/3600) ≈ 486000+`, i.e. exactly the class of bug reported (`495740:07:43`).
- `TimestampLabel`/`Timestamp.svelte` has effectively one real caller in the app (`InlineNote.svelte:57`; the only other reference is the design-system showcase page `src/routes/design-system/+page.svelte`), so this is a straightforward type/unit mismatch: the component is a duration formatter being used as a wall-clock formatter.
- A fix would concretely touch one of: (a) `Timestamp.svelte` — change `format()` to render actual wall-clock time (e.g. `new Date(ms).toLocaleTimeString(...)`, matching the pattern already used in `FeatureTourStep.svelte:38-45`), or (b) `InlineNote.svelte`/`DictatePracticeStep.svelte` — stop feeding an epoch ms into a duration formatter (pass a formatted `timestampLabel` string instead, using the same `toLocaleTimeString` pattern). Given there's essentially one live consumer, either fix is safe; (a) is more correct since the component's name/prop (`TimestampLabel`, `at`) imply wall-clock, not duration.
- Size estimate: trivial. Single-file, few-line fix; no backend involvement.
