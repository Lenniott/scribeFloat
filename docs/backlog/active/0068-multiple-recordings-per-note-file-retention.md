---
id: "0068"
title: Clarify and test multi-segment recording behaviour with file retention enabled
status: active
---

# Clarify and test multi-segment recording behaviour with file retention enabled

As a user with "Keep markdown file" and "Keep WAV file" enabled, I want to understand what happens when I record multiple segments to the same note so that I'm not surprised by unexpected files or overwritten data.

The current flow: each `scribe_stop_and_save` produces a transcript, then `note_attach_transcript` appends it to the note's markdown content. If file retention is on, a sidecar `.md` and `.wav` are written per recording. It is unclear whether these sidecar files are keyed by note ID (which would cause overwrites on the second recording) or by a unique recording/session ID (which would accumulate files correctly).

## Notes

- Investigate `note_attach_transcript` and the file-retention logic in the Rust backend to confirm the file-naming scheme.
- If sidecar files are overwritten on each recording to the same note, this is a data loss bug that should be promoted to a bug fix story.
- If files accumulate correctly, add a brief explanation to the settings UI so users know what to expect.
- The note editor content (in-app markdown) is appended on each attach — verify this is also working correctly for the multi-segment case (separator between segments, timestamps, speaker labels).
