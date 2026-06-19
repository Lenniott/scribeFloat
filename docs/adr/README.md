# ADR Index

| ADR | File | Decision |
|-----|------|----------|
| ADR-0001 | [0001-note-as-primary-domain-object.md](0001-note-as-primary-domain-object.md) | The Note is the primary domain object — replaces HistoryRecord as the central concept |
| ADR-0002 | [0002-note-is-a-composition-of-sources.md](0002-note-is-a-composition-of-sources.md) | A Note is composed of one or more Sources, each with a type and content |
| ADR-0003 | [0003-scribe-and-dictate-are-capture-profiles.md](0003-scribe-and-dictate-are-capture-profiles.md) | Scribe and Dictate are capture profiles of a shared model, not separate concepts |
| ADR-0004 | [0004-triage-is-per-note-not-per-flow.md](0004-triage-is-per-note-not-per-flow.md) | Triage status lives on the Note, not on the capture flow that produced it |
| ADR-0005 | [0005-knowledge-layer-stored-as-markdown-not-database.md](0005-knowledge-layer-stored-as-markdown-not-database.md) | The knowledge layer is stored as Markdown files, not a database |
| ADR-0006 | [0006-separate-capture-config-from-note-intent.md](0006-separate-capture-config-from-note-intent.md) | Capture is intake configuration only; "quick" is the only capture-derived Note property in the UI |
