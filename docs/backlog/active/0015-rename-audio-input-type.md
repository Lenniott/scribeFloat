---
id: "0015"
title: Rename audio_input_type — replace "source" meaning mic/speaker
status: active
adr: ADR-0002
---

# Rename audio_input_type in code

Replace any use of "source" to mean mic/speaker with `audio_input_type: mic | speaker`. Frees the term "source" for the Note Source concept (ADR-0002).

## Why

"Source" currently means two different things: the audio input device (mic vs speaker) and a Note Source (transcript, written, upload_audio, etc.). This rename eliminates the collision.
