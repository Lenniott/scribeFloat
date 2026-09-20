# Follow-ups

- [Slow Note saving](issues/01-investigate-slow-note-save.md): saving still delays
  HUD dismissal and next-recording availability even though paste is earlier.
- Broader streaming accuracy: the original 15-second fixture matches baseline,
  but a synthetic repeated long sequence varies around “rapping/wrapping” and
  repeated words. Collect varied, human-transcribed natural speech before making
  accuracy claims or reducing the 30-second minimum context. Existing repeated-
  block cleanup makes the synthetic sequence unsuitable as a final-text WER test.
- Live app smoke check: normal dictation, cancellation, and paste after the
  background-ASR integration have not been exercised with a physical microphone
  by the agent; offline worker lifecycle and ASR replay tests are covered.

- Record now uses capture-time mic ASR with final speaker alignment. Native smoke
  checks remain: two voices, microphone switching, system-audio toggling, Stop
  cancellation, WAV-only save, and attaching to an existing Note. Unit tests do
  not establish real-world diarization accuracy or a Record latency improvement.
- System-audio ASR remains post-Stop; only the mic is processed during capture.
  Extending loopback requires preserving recording-relative offsets across
  enable/disable cycles and testing the combined worker load.

- Windows readiness explicitly deferred by the user during macOS merge prep.
  The earlier unconditional Unix import in cli_link.rs remains a Windows build
  blocker; macOS-only validation does not establish Windows CI success.
- Packaged macOS validation used a native unsigned debug app and release CLI.
  Intel, signing/notarization, and actual live capture remain unverified.
