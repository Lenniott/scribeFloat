# Handoff — context-chunking-strategy

**State right now**: `issues/01-remove-dead-pitch-loudness-analyzer.md` is
done — the dead pitch/loudness cut-detection machinery (ADR-0013) is removed
from `src-tauri/src`, ADR-0013 carries a removal note, `docs/adr/README.md`
is updated. `cargo check` / `cargo clippy -p ScribeFloat -- -D warnings` /
`cargo test -p ScribeFloat` all pass (347 passed, 5 ignored hardware-gated —
ignore that count if it moves, it's unrelated hardware-gated tests). Not yet
committed as of this handoff.

**What's next on the frontier**: everything else on `MAP.md`'s "Decisions so
far" (chunk boundary policy, `ContextChunk` schema fork, silence-triggered
ASR chunking) is still exploration-stage — needs a session to turn each into
a concrete ticket before an agent should touch code.

**Don't re-discover**: the grep evidence that motivated issue 01 (call sites,
what was dead vs. what stayed live via `rms()`) doesn't need re-proving —
that work is done and verified, not just planned.
