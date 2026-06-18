---
id: "0019"
title: Deprecate NotePanel / NoteComposer / NoteCard
status: active
adr: ADR-0001
---

# Deprecate NotePanel / NoteComposer / NoteCard

These components implement the chat-style note-taking pattern being replaced by a unified editable Note body. Do not extend them — replace usage as screens are rebuilt.

Mark each with a `<!-- DEPRECATED -->` comment pointing to the replacement pattern.
