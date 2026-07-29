---
title: "Triage: Record vs Dictate naming / dual-controller honesty"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

"Record" and "Dictate" are presented to users as one product surface, but the codebase still has two large, genuinely divergent controllers (`ScribeController` 1777 lines, `DictateController` 1489 lines) that ADR-0003 already flagged as an artefact of build order and deferred unifying, plus scattered leftover "Scribe" strings in live UI (notes filter chip, onboarding copy, settings labels) that never got renamed to "Record".

## Question

Read the "Record vs Dictate naming / dual-controller honesty" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. Two separable pieces: (1) cosmetic "Scribe"→"Record" string rename in ~6 UI spots — small, low-risk, could be Now; (2) controller unification behind a shared `CaptureProfile` per ADR-0003 — large, architecturally risky, already deferred once. Do the cosmetic rename Now and leave unification Later (its own wayfinder), or defer both together?

## Findings

**ADR-0003** — `docs/adr/0003-scribe-and-dictate-are-capture-profiles.md`, status Binding, wayfinder "Implemented — Main is God again". Decision: Record and Dictate use identical underlying tech (cpal capture, bundled Whisper Small, Note creation); their only real differences are configuration — audio durability, stop safeguard, activation method, output destination. The two-controller split (`ScribeController` / `DictateController`) is explicitly named as "an artefact of build order... not a domain distinction," and future unification into a single capture system parameterised by a `CaptureProfile` is deferred, not rejected — deferred because breaking the audio pipeline mid-refactor is judged too risky. Consequence clause: don't widen the gap between the two controllers going forward.

**Backend controllers** (both `Arc`-wrapped, mutex-guarded `Inner`, own state machine + tests):
- `ScribeController` — `src-tauri/src/controllers/scribe.rs` (1777 lines). Owns: `start`/`stop_and_save`/`cancel`/`save_recording_only`/`abort_transcription_keep_wav`, mic device switching (`switch_mic`, `list_input_devices`/`list_output_devices`), speaker-diarization accumulation (`SpeakerAccumulator`, `SpeakerSegment`, session manifest read/write), timestamp toggle, transcript listing/reading, note attachment (`set_attach_note`/`add_note`).
- `DictateController` — `src-tauri/src/controllers/dictate.rs` (1489 lines). Owns everything `ScribeController` does for the capture→transcribe→write path (`start`, `stop_and_transcribe`, `cancel`, `abort_processing`, `do_transcription`, shutdown finalize) **plus** a hotkey state machine (`DictateKeyTracker`: first-press/second-press/hold/toggle/cooldown/timeout handling — ~15 unit tests), auto-dismiss/paste-to-active-app logic (`paste_on_main_thread`, `hide_window`), and error salvage (`salvage_dictate_wav`).
- Overlap: both implement near-identical `start`, `cancel`, `do_transcription`, `finalize_capture_on_shutdown`, `spawn_record_start_preload` — this is the duplicated "shared infrastructure" ADR-0003 says should exist but doesn't. Divergence: Dictate additionally owns hotkey arming/dispatch and paste-out; Scribe additionally owns speaker diarization and richer transcript/session-manifest management. Neither is a strict superset — real behavioral divergence, not just naming.
- Command layer mirrors the split 1:1: `src-tauri/src/commands/scribe.rs` (`scribe_start`, `scribe_cancel`, `scribe_add_note`, etc.) and `src-tauri/src/commands/dictate.rs` (`dictate_trigger`, `dictate_cancel`, `dictate_get_state`, etc.) — both wired in `src-tauri/src/lib.rs`, `src-tauri/src/commands/settings.rs`, `src-tauri/src/commands/history.rs`.
- Frontend: `src/lib/stores/scribeController.svelte.ts` (303 lines) is the store wrapping the Scribe/Record side; no equivalent `dictateController.svelte.ts` file was found under `src/lib/stores` (dictate state appears handled elsewhere/differently on the frontend — worth confirming before scoping a fix).

**"Scribe" naming remnants in live (non-prototype) UI**:
- `src/lib/ui/5_views/notes.svelte:17` — filter chip `{ id: 'scribe', label: 'Scribe' }`
- `src/lib/ui/5_views/notes.svelte:107` — empty-state copy `'No Scribe notes yet.'`
- `src/lib/ui/5_views/notes.svelte:118` — badge `{ label: 'Scribe', variant: 'brand' }`
- `src/lib/ui/5_views/notes.svelte:188` — help copy "Every note — Scribe, Dictate, Upload, and written."
- `src/lib/ui/4_sections/onboarding/FeatureTourStep.svelte:18` — onboarding tour label `"Scribe"`
- `src/lib/ui/4_sections/onboarding/WelcomeStep.svelte:28` — welcome screen label `"Scribe"`
- `src/lib/ui/5_views/setting_general.svelte:59,71,106,217` — internal var/fn names (`scribeCaptureSpeaker`, `setScribeCaptureSpeaker`) not user-facing text, lower priority.
- `src/routes/design-system/+page.svelte` (multiple lines ~87–1263) — "Scribe" appears extensively but this is the internal design-system reference page, not shipped product UI.

**Scope estimate**: Not a mechanical rename. A pure string/identifier find-replace (`Scribe`→`Record`) would be quick, but ADR-0003 already describes the real work as unifying two controllers with genuinely divergent responsibilities (hotkey state machine + paste-out vs. speaker diarization + session manifests) behind a shared `CaptureProfile` abstraction, without breaking the live audio pipeline. That is a structural refactor across `src-tauri/src/controllers/{scribe,dictate}.rs` (~3266 lines combined), their command layers, and frontend state — plus a separate, smaller pass to rename user-facing "Scribe" strings to "Record". These are two different-sized efforts that happen to share a name: (1) cosmetic rename in ~6 UI spots — small, low-risk, could be done standalone; (2) controller unification per ADR-0003 — large, architecturally risky, explicitly deferred already once. Confirms the original note: deeper than a quick rename, and the unification half genuinely warrants its own future wayfinder rather than folding into this effort. The cosmetic rename half could optionally be peeled off as a quick Now-item if the human wants visible naming consistency without touching controllers.
