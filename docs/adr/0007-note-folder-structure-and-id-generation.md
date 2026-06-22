# ADR-0007: Note folder structure and ID generation

## Status

Accepted

## Context

Currently, Scribe sessions produce a staging folder that is deleted after transcription (when `keep_wavs` is off). Transcript markdown files land in the save folder root with a flat `title_model.md` naming scheme. This makes it impossible to co-locate a Note's written source, transcript, audio, and metadata in one place, and it produces messy flat directories as the user accumulates notes. A durable folder per Note is needed to support the unified note editor and the `written` source type.

Additionally, a stable, short, human-readable identifier is needed for each Note folder — one that does not require a database and can be derived deterministically from the Note's creation timestamp and title.

## Decision

**Every Note gets a dedicated folder.** All artifacts for that Note live inside it: the written source (`.md`), transcript segments or rendered markdown, audio files, and future attachments.

**Folder naming convention:**

```
HHMM_DD-MM-YY_first_seven_title_words_XXXXXX
```

- `HHMM` — creation time (24-hour, local)
- `DD-MM-YY` — creation date
- `first_seven_title_words` — the first seven alphabetical words of the title, lowercased, joined with underscores (non-letter characters used as word delimiters, empty tokens dropped)
- `XXXXXX` — 6-character base-36 ID derived by taking the MD5 hash of `"{unix_timestamp} {7-word-title}"`, interpreting it as a base-16 integer, taking modulo 36⁶, and encoding in base-36 (uppercase, zero-padded)

**Title cap:** Note titles are capped at 7 words for the purpose of folder naming. The display title is uncapped.

**Example:** A note created at 10:30 on 04/06/2025 titled "Daily-standup meeting!! for the core team" →
`1030_04-06-25_Daily_standup_meeting_for_the_core_team_A3BX9Y`

**Audio retention:** Staging WAV files are retained inside the Note folder after processing (not deleted). The current `keep_wavs` setting is superseded for notes that use this folder structure — all audio is kept.

**Markdown export:** When auto-save-to-markdown is enabled, the rendered note (written source + transcript body, YAML frontmatter with metadata) is written to `note.md` inside the Note folder. The YAML frontmatter includes title, tags, creation timestamp, word count, model, and duration.

## Consequences

- Notes are self-contained on disk — the folder is the unit of portability, backup, and deletion
- The existing flat `title_model.md` export path is retired for new notes; existing files are unaffected
- The `session_dir` concept in `ScribeController` is replaced by the Note folder — audio streams directly into it
- ID generation is a pure function (timestamp + title → ID) with no database required; collisions within the same second on the same title are prevented by the timestamp component
- Folder names are human-readable and sortable by time without tooling
- The 6-char base-36 ID space is 36⁶ = ~2.2 billion values; practical collision risk within a single user's library is negligible
