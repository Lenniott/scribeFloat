# Note as the primary domain object

**Status:** Binding
**Wayfinder:** Implemented — Main is God again / current product. Residual naming debt: canonical Rust type remains `HistoryRecord`; a small session-annotation struct is also named `Note`.

The existing `HistoryRecord` (a JSONL entry in `history.jsonl`) was the closest thing to a primary entity, but the name described the storage mechanism, not what the thing *is*. We renamed it **Note** — a piece of knowledge the user owns, edits, and builds on over time. This reflects the product direction: ScribeFloat is not a history log, it is a personal knowledge system. Every capture method (Scribe, Dictate, Upload) produces a Note; Float enriches Notes; the knowledge layer synthesises across Notes.

**Why this is not a simple rename:** Note carries a different contract than HistoryRecord. HistoryRecord was append-only and archival. A Note is mutable — both the user and Float can edit its body and metadata. That changes the storage and concurrency model.

**Considered alternatives:** Keeping HistoryRecord as the canonical term and mapping it to Note in the UI only. Rejected because it would leave a permanent split between how the codebase talks about the entity and how every other document (CONTEXT.md, PRDs, agent prompts) talks about it — agents would constantly misfire when the term they read doesn't match the term in the code.
