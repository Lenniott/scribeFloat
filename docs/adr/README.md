# ADR Index

| ADR | File | Decision |
|-----|------|----------|
| ADR-0001 | [0001-note-as-primary-domain-object.md](0001-note-as-primary-domain-object.md) | The Note is the primary domain object — replaces HistoryRecord as the central concept |
| ADR-0002 | [0002-note-is-a-composition-of-sources.md](0002-note-is-a-composition-of-sources.md) | A Note is composed of one or more Sources, each with a type and content |
| ADR-0003 | [0003-scribe-and-dictate-are-capture-profiles.md](0003-scribe-and-dictate-are-capture-profiles.md) | Scribe and Dictate are capture profiles of a shared model, not separate concepts |
| ADR-0004 | [0004-triage-is-per-note-not-per-flow.md](0004-triage-is-per-note-not-per-flow.md) | Triage status lives on the Note, not on the capture flow that produced it |
| ADR-0005 | [0005-knowledge-layer-stored-as-markdown-not-database.md](0005-knowledge-layer-stored-as-markdown-not-database.md) | The knowledge layer is stored as Markdown files, not a database |
| ADR-0006 | [0006-unified-note-editor-replaces-scribe-and-detail.md](0006-unified-note-editor-replaces-scribe-and-detail.md) | Unified note editor at /notes/[id] replaces Scribe panel and NoteDetailPane |
| ADR-0007 | [0007-note-folder-structure-and-id-generation.md](0007-note-folder-structure-and-id-generation.md) | Note folder named HHMM_DD-MM-YY_title_XXXXXX; 6-char base-36 ID from MD5 hash |
| ADR-0008 | [0008-codemirror-for-written-source-editor.md](0008-codemirror-for-written-source-editor.md) | CodeMirror 6 for the written source editor panel; source-mode markdown only |
| ADR-0009 | [0009-note-lifecycle-immediate-create-autosave-discard-if-empty.md](0009-note-lifecycle-immediate-create-autosave-discard-if-empty.md) | Note created immediately on open; autosaved; silently discarded if empty on leave |
| ADR-0010 | [0010-separate-capture-config-from-note-intent.md](0010-separate-capture-config-from-note-intent.md) | Capture is intake configuration only; "quick" is the only capture-derived Note property in the UI |
| ADR-0011 | [0011-tags-replace-tags-and-keywords.md](0011-tags-replace-tags-and-keywords.md) | Keywords removed; Tags is the single vocabulary concept; each Tag carries per-note annotation logs written by Float |
