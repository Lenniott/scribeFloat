---
title: "Triage: Onboarding Dictate practice pays cold Whisper load"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Onboarding Dictate practice pays cold Whisper load" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

- `spawn_record_start_preload` (`src-tauri/src/controllers/dictate.rs:609-618`) does `model.preload_context(&path)` in a blocking task, but it's only invoked from `dispatch_action` at `dictate.rs:426`, which fires at key-down/HUD-request time — i.e. the moment the user actually triggers a dictate action. Comment at `dictate.rs:423-426` confirms this is intentional ("as early as possible (key-down/HUD-request time)").
- Reproduces as written: nothing calls `spawn_record_start_preload` (or `model.preload_context` directly) during onboarding before the user first double-taps in `DictatePracticeStep.svelte`. Onboarding flow (`src/lib/ui/5_views/onboarding.svelte:13,84-86`) goes Welcome → Permissions (mic granted at `PermissionsStep.svelte:24,113-123`, gating `Continue`) → DictatePracticeStep (mount at `DictatePracticeStep.svelte:56-77` only sets up an event listener and fetches auto-enter setting — no preload call) → FeatureTour. So the first practice recording is the first thing that ever calls Whisper preload.
- Idle window confirmed: after mic permission is granted (`PermissionsStep` `Continue` enabled) and before the user actually double-taps to record in `DictatePracticeStep`, there's dead time (step transition + reading "How to use" instructions) where a preload could be kicked off, e.g. in `DictatePracticeStep.svelte` `onMount` (`:56`) or right when `PermissionsStep`'s `onNext` fires once mic is granted.
- A fix would concretely touch: either add a Tauri command that calls the dictate controller's model preload (reusing `DictateController::spawn_record_start_preload`, which needs to become `pub`/exposed, or a new thin wrapper) and invoke it from `PermissionsStep.svelte` once `micGranted` flips true, or from `DictatePracticeStep.svelte`'s `onMount`. Backend side is `src-tauri/src/controllers/dictate.rs:609-618`; frontend side is `PermissionsStep.svelte` or `DictatePracticeStep.svelte` plus wiring a new/reused Tauri command.
- Size estimate: small. Preload logic already exists and is decoupled from mic/audio state per its own doc comment (`dictate.rs:606-608`); this is mostly "call it earlier" plus exposing a command.
