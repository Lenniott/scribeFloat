# Scribe and Dictate are capture profiles, not distinct systems

Scribe and Dictate use the same underlying technology: audio capture via cpal, Whisper transcription, Note creation. Their differences are entirely configuration: audio durability (durable folder vs temp file), transcription model (better vs faster), stop safeguard (confirmation prompt vs none), activation (in-app UI vs global hotkey), and output destination (in-app Note vs paste to active app).

The current two-controller architecture (`ScribeController`, `DictateController`) is an artefact of build order — Dictate was added after Scribe — not a domain distinction. Future refactoring should unify them into a single capture system parameterised by a capture profile. This has not been done yet because the risk of breaking the audio pipeline mid-session outweighs the immediate gain; it is captured here so the split is not mistaken for intentional architecture and deepened further.

**Consequence:** Do not add new features that widen the gap between the two controllers (e.g. a capability that only works in one because it was "easier to add there"). Any new recording capability belongs in shared infrastructure.
