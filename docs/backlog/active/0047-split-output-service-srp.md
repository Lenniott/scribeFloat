---
id: "0047"
title: Split OutputService into focused sub-modules (SRP)
status: complete
priority: medium
---

# Split `OutputService` into focused sub-modules

## Problem

`src-tauri/src/services/output.rs` is ~1,900 lines and contains at least **six unrelated responsibilities**:

| Responsibility | Representative symbols |
|---|---|
| WAV header management | `sync_wav_header`, `repair_wav_header_from_file_size`, `write_streaming_wav_placeholder` |
| Text deduplication | `dedup_exact_halves`, `dedup_consecutive_phrases`, `dedup_repeated_block` |
| Text cleanup & replacement | `cleanup_text`, `apply_replacements`, `replace_phrase`, `replace_whole_word`, `replace_newline`, `wrap_next_word` |
| Transcript rendering | `render_transcript_markdown`, `render_transcript_body`, `write_transcript` |
| Session lifecycle | `finalize_scribe_session`, `make_session_dir`, `remove_session_dir` |
| File utilities & legacy listing | `read_transcript`, `delete_file`, `salvage_dictate_wav`, `list_transcripts`, `list_transcript_metadata` |

A god file at this scale violates SRP and makes it hard to read, test, or change any single concern without scrolling past unrelated code. Callers import `OutputService` and get all six responsibilities whether they need them or not (ISP violation).

## Goal

Decompose `output.rs` into a set of **focused sub-modules** under `src-tauri/src/services/output/`. Each sub-module owns one concern. The public surface of `OutputService` is preserved — callers in `controllers/` should not need to change.

## Target module structure

```
src-tauri/src/services/output/
  mod.rs          ← re-exports; OutputService struct; thin delegation methods
  wav.rs          ← WAV header write/patch/repair
  dedup.rs        ← dedup_exact_halves, dedup_consecutive_phrases, dedup_repeated_block
  text.rs         ← cleanup_text, apply_replacements, replace_phrase, replace_whole_word, replace_newline, wrap_next_word
  render.rs       ← render_transcript_markdown, render_transcript_body, write_transcript
  session.rs      ← finalize_scribe_session, make_session_dir, remove_session_dir
  legacy.rs       ← read_transcript, delete_file, salvage_dictate_wav, list_transcripts, list_transcript_metadata
```

`mod.rs` imports from each sub-module and exposes only the methods `OutputService` needs to call. Private helpers stay private inside their sub-module.

## Implementation steps

1. Create `src-tauri/src/services/output/` directory.
2. Move `src-tauri/src/services/output.rs` to `src-tauri/src/services/output/mod.rs` (git mv or create+delete).
3. For each sub-module, extract the relevant functions into a new `*.rs` file. Mark helpers `pub(super)` if needed by `mod.rs`, otherwise keep them `fn` (private).
4. Update `mod.rs` to `mod wav; mod dedup; mod text; mod render; mod session; mod legacy;` and `use` the symbols each delegation method needs.
5. Ensure `src-tauri/src/services/mod.rs` still exposes `pub mod output;` — no change required there.

## Constraints

- **Zero behaviour change.** This is a pure structural refactor.
- **Do not change any public method signatures** on `OutputService` — controllers call these directly.
- **Tests stay in the same sub-module as the code they test** (`#[cfg(test)]` blocks move with their functions).
- **Do not add new abstractions** (traits, generics, new structs). Flat free-function modules are fine.
- Apply story 0046 (LazyLock regexes) in `text.rs` as part of this work if 0046 has not already landed.

## Acceptance criteria

- [ ] `src-tauri/src/services/output.rs` no longer exists; `src-tauri/src/services/output/mod.rs` exists.
- [ ] Each of the six sub-modules exists as a separate file.
- [ ] No file in the new module tree exceeds 400 lines.
- [ ] All call sites in `controllers/` compile without modification.
- [ ] All existing tests in the former `output.rs` still pass in their new locations.
- [ ] `cargo test -p scribefloat` passes.
- [ ] `cargo clippy -- -D warnings` passes.

## Notes

- Use `git mv src-tauri/src/services/output.rs src-tauri/src/services/output/mod.rs` to preserve git blame history before splitting.
- If a function is called by more than one sub-module, keep it in the sub-module that owns its primary concern and `use super::text::cleanup_text` from others — avoid duplication.
- `dedup.rs` and `text.rs` are pure functions with no IO; they are the best candidates for focused unit tests.
