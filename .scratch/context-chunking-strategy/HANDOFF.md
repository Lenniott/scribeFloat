# Handoff — context-chunking-strategy

**State right now**: pure exploration, nothing built. No code has changed;
this effort folder itself was created in this session to stop the exploration
from evaporating when the thread ends.

**What's next on the frontier**: `issues/01-remove-dead-pitch-loudness-analyzer.md`
is fully specified and ready to pick up. Everything else on `MAP.md`'s
"Decisions so far" (chunk boundary policy, `ContextChunk` schema fork,
silence-triggered ASR chunking) is still exploration-stage — needs a session
to turn each into a concrete ticket before an agent should touch code.

**Don't re-discover**: the grep evidence in issue 01 (call sites, what's dead
vs. what's live via `rms()`) was exhaustive at time of writing — re-verify
rather than re-deriving from scratch if the codebase has moved since
2026-08-13, but don't assume the negative result needs re-proving from zero.

**Nothing dirty in the working tree** — this session only wrote files under
`.scratch/context-chunking-strategy/`.
