# Knowledge layer Artifacts stored as markdown files, not a database

**Status:** Aspirational
**Wayfinder:** pre-wayfinder / orphan — knowledge layer not built; revisit in a future wayfinder (out of scope for Main is God again).

Domains and Artifacts (the knowledge layer above Notes) are stored as markdown files with YAML frontmatter in a folder structure — not in a database or a separate structured store. A Domain is a folder; an Artifact is a `.md` file with `type:`, `title:`, `tags:`, and `sources:` frontmatter.

This was chosen because the knowledge layer is fundamentally a document system: Artifacts are meant to be read by humans, edited by both humans and agents, handed to stakeholders, and kept in version control. A database would require a server process, a query layer, and bespoke tooling to read or edit — all of which fight the local-first, human-readable goal. The OKF (Open Knowledge Format) pattern validates this: markdown + YAML frontmatter is an established, portable, agent-readable convention that needs no SDK to consume.

**Consequence:** Artifact search and cross-linking is done by scanning frontmatter, not by SQL queries. This scales to thousands of files (one designer's corpus) without issue. If the corpus grows to tens of thousands of Artifacts and scan performance degrades, a lightweight index (a generated `index.md` per Domain, or a single manifest file) is the first remedy — not a database migration.
