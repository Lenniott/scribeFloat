# Investigate slow Note saving after Dictate

Status: needs-triage

Separate from moving paste ahead of persistence, as requested by the user.

## Evidence

On 2026-09-17, dictation run `89224325-b113-4303-b726-5c8eecd05402`
recorded 21.59 seconds of audio. Transcription/formatting took 2216 ms, but the
history append plus `note://item-added` emission took 11877 ms. Total
Stop-to-paste was 14810 ms. Log: `/tmp/scribefloat-dictate-timing.log`.

After moving paste first, run `59aa0c5f-5a8b-413c-8a28-de6760c27891` recorded
314.67 s audio. Paste acknowledged at 16329 ms, followed by a 15003 ms history
checkpoint. No history-write error logged; worker finished at 31778 ms. Slow
saving recurs and still delays HUD dismissal/next recording.

## Investigation

Measure history mutex acquisition, first-load JSONL parsing/sidecar hydration,
record serialization, file append/flush/sync, and event emission independently.
Compare cold and warm saves. No individual substep is yet established as the
cause; do not remove durability guarantees based on this aggregate measurement.

Paste now precedes history saving, but the HUD and next-dictation availability
still wait for persistence in the existing worker. Address that residual wait
when investigating saving, without introducing untracked saves or late events
that overwrite a newer session's state.
