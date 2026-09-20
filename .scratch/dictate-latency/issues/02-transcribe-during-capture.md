# Transcribe long Dictate sessions during capture

Status: implemented — offline validation complete

User authorized using `dicate-test.wav` to benchmark and implement lower Dictate
latency. Slow Note persistence remains issue 01; the earlier paste-first change
is retained.

## Implementation

Shared `StreamingTranscription` uses the writer-thread PCM tap, a bounded channel,
and a CPU Silero detector. It closes disjoint spans after at least 30 seconds and
512 ms of confirmed silence. PCM is quantized identically to the saved WAV.
Inference uses the existing Whisper model/settings and global inference gate.
Stop joins the worker, formats its segments through shared Dictate assembly, and
pastes before saving. Successful live ASR avoids rereading the WAV. Failures or
overload discard all partial results and replay the complete WAV; cancellation
never publishes partial text. ADR-0016 records the architectural decision.

## Validation scope

Original fixture: 15 seconds, mono int16 at 44.1 kHz; resample through the capture
resampler. Also repeat it four times with one-second gaps for a labelled synthetic
63-second scheduling test. Repetition invokes existing text-deduplication behavior,
so compare raw segments as well as final text; this is not a natural-speech WER
corpus. No human reference transcript has been supplied.

Reject 4-second minimum/256 ms pause configuration: ~1.10 s to ~0.88 s remaining
on the short clip, but it changed "rapping" to "wrapping". Text prompt carryover
did not repair it. Flash attention also changed words and is not enabled. Removing
the inference VAD pass did not resolve the long repeated fixture's variation.
The final configuration keeps one job for the original 15-second recording.

Automated tests cover capture-time inference before Finish, final-tail order,
sample conservation across arbitrary packet sizes, cancellation, inference error,
queue overflow, continuous speech reaching the bound, and WAV-equivalent samples.
Physical microphone/paste behavior still needs a normal dev-app smoke check after
these offline tests; the benchmark never opens a mic or writes the clipboard.

## Result

See [benchmark results](../BENCHMARK.md): final original clip text matches exactly;
the synthetic 63-second replay reduces remaining ASR from 4.153 s to 2.135 s.
Long-fixture wording differences are disclosed in the report. Do not describe
the scheduling tests as general transcription-accuracy validation.
