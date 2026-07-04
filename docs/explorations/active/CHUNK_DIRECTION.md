# Speaker-Aware Chunking for Transcription — Direction, Logic, Success Criteria

## What we are building and why

We transcribe multi-speaker conversations with a speech-to-text model that works
on chunks. Today chunks are cut at silence gaps. That fails in real conversation:
speakers often hand over with no measurable pause, so one chunk ends up containing
two speakers, which hurts the transcript and everything downstream.

Goal of this stage: **cut chunks at speaker handovers, not just at silences.**
This stage only marks *where* the voice changes. It does NOT decide *who* is
speaking — a separate speaker-identification step runs later on the marked
chunks. Do not build identification into this stage.

Working assumptions (validated as acceptable): in a conversation each person
keeps a fairly consistent voice height, and stays at a roughly constant distance
from the microphone. Deliberate voice acting and impersonation are accepted edge
cases.

## The logic (validated by experiment)

1. **Track the voice, not the waveform.** Extract, over time: (a) how high the
   voice is (its fundamental tone), (b) how loud the speech is. Only measure
   these on moments that actually contain voiced speech — silences, breaths,
   music and consonants must be excluded or they poison everything.

2. **Compare "before vs after" using speech time, not clock time.** At each
   candidate moment, compare a summary (use a robust middle value, not an
   average) of the last ~1 second of *voiced speech* before it against the next
   ~1 second of voiced speech after it. Windows must skip over silence: real
   handovers usually sit inside a pause, and clock-time windows go blind exactly
   there. This was our biggest bug.

3. **Measure voice-height changes on a musical/log scale, not in raw
   frequency.** A fixed frequency difference is huge for a deep voice and
   trivial for a high one. On a log scale one threshold works for everyone.

4. **Set thresholds from the speaker's own variation.** Measured on real
   recordings: while one person talks, their voice height wobbles about 2–3
   log-steps (semitones) between one-second windows, and sentence melody swings
   further moment-to-moment. A change flag is only meaningful when the jump is
   clearly above that self-variation (we used >4 semitones for voice height,
   >6 dB for loudness). Anything below the self-variation floor is
   mathematically undetectable — don't tune, don't try.

5. **Combine independent signals; they fail on different handovers.** Observed
   directly: voice height catches changes loudness misses and vice versa
   (e.g. two men at the same tone but recorded at different levels). Silence
   gaps, voice-height jumps and loudness jumps should all produce cuts, as a
   union. Extra cuts are harmless for transcription — a missed handover is the
   only expensive error. Bias every decision toward over-cutting.

6. **Silence stays in the toolbox but is not trusted alone.** On edited audio
   with gaps at every join, silence alone found every boundary. On tightly cut
   audio it found zero. It detects pauses, not people.

7. **Attach evidence to each cut.** Record the size of the voice-height and
   loudness jump at every cut. Downstream, big-jump cuts are near-certain
   speaker changes and small-jump cuts are near-certain same-speaker pauses —
   this prioritizes the later identification step's work.

## Known limits (so nobody rediscovers them)

- Two voices less than ~2 log-steps apart in height AND similar in loudness are
  indistinguishable to this stage. In our benchmark, 1 of 5 handovers was of
  this kind. The later identification step owns that case.
- One person deliberately shifting their voice creates false change points.
  Accepted edge case; over-cutting is harmless here too.
- The tone tracker pins to its measurement floor on music/rumble — readings at
  the floor of the measured range must be discarded, not believed.

## Success criteria

Test on recordings with known handover times (keep a benchmark file whose true
boundary times are written down; when the audio file changes, its truth times
must change with it — we lost an hour to a silently swapped test file).

1. **Recall is the headline number:** on conversations where consecutive
   speakers differ audibly in voice height or level, ≥4 of 5 true handovers get
   a cut within 1 second. (Achieved: 4/5 at 0.25 s average error on a tight-cut
   benchmark where silence-only scored 0/5.)
2. **No chunk may contain two speakers** for boundaries the signals can see;
   chunks that mix speakers are counted and reported — target zero visible ones.
3. **Chunk lengths must fit the transcriber:** no chunk longer than its input
   limit (~30 s); over-cutting to stay under is fine.
4. **Must beat the silence-only baseline** on tight-edited/live audio while not
   doing worse when gaps do exist (run both, union them).
5. **Output must be checkable by a non-technical person:** a timeline in
   minutes:seconds — where the cuts are, how strong the evidence is at each, in
   plain words — so it can be verified against known content by listening, not
   by reading statistics.
