# A Note is a composition of Sources, not a flat text blob

**Status:** Aspirational
**Wayfinder:** pre-wayfinder / orphan — revisit before treating as active intent. Partial today: written + transcript coexist flatly on `HistoryRecord`; no `sources: Vec` model yet.

A Note is not a single markdown string. It is composed of one or more **Sources** — individually addressable content pieces, each with a type and origin (`transcript`, `written`, `upload_audio`, `web`, `video`, `import_md`). Float can process Sources separately or together; agents reading a Note do not have to consume all context at once.

This was chosen over a flat text body because the product roadmap requires Notes built from multiple origins (a Scribe recording + written notes, or an imported audio file + a web scrape). A flat body would require merging content at write time, making it impossible to later distinguish what came from where — which matters for Float's per-Source processing and for the knowledge layer's source-linking.

**Naming note:** "Source" was chosen over "Block", "Chunk", "Attachment", and "Piece". "Chunk" was rejected because it is already the term used in Float for how text is split for LLM calls. "Source" describes origin, which is the dimension that matters. The audio-input distinction (mic vs speaker) was renamed to `audio_input_type: mic | speaker` to free "source" for this domain use.
