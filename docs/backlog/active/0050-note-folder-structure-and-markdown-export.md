---
id: "0050"
title: Note folder structure and markdown export
status: active
adr: ADR-0007
---

# Note folder structure and markdown export

Implements the per-note folder naming convention from ADR-0007 and extends auto-save-to-markdown to cover full notes (written source + transcript + YAML frontmatter). Written notes get their own folder on creation; the markdown file (`note.md`) is kept in sync with every autosave.

Depends on: 0049 (note lifecycle, where `note_create_empty` is the creation point). Build after 0049.

---

## Interim storage (until this story ships)

Editor autosave currently uses `{save_folder}/.notes/{uuid}/written.md` + `meta.json` via `services/note_sidecar.rs`. **0050 replaces** that layout with ADR-0007 folder names and optional `note.md` export — migrate paths in `note_sidecar`, do not add a third layout.

---

## Backend

### 1. Add `md5` crate to `src-tauri/Cargo.toml`

```toml
md5 = "0.10"
```

### 2. Add `note_folder_name` function in a new `src-tauri/src/services/note_folder.rs`

This is a pure function — no I/O, no state. Keep it in its own file for testability.

```rust
/// Returns `HHMM_DD-MM-YY_word1_word2_..._XXXXXX` for a given note.
///
/// `created_at_unix`: Unix timestamp (seconds) of the note's creation time.
/// `title`: Full display title (uncapped).
///
/// ID generation (port of the Python reference in ADR-0007):
///   1. Extract up to 7 alphabetical words: split on non-alphabetic chars, take first 7.
///   2. Join words with underscores: `word1_word2_..._word7`
///   3. Input to MD5: `"{created_at_unix} {joined_words}"`
///   4. MD5 digest → base-36 (0-9a-z), take first 6 chars, uppercase.
///   5. Timestamp prefix: `chrono::DateTime::from_timestamp(created_at_unix, 0)` → `%H%M_%d-%m-%y`
///   6. Result: `{timestamp_prefix}_{joined_words}_{6char_id}`
pub fn note_folder_name(created_at_unix: i64, title: &str) -> String
```

Base-36 encoding: `const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";` — take the MD5 u128 value, repeatedly divide by 36 and map remainders, take first 6 characters.

Add `mod note_folder;` and `pub use note_folder::note_folder_name;` to `src-tauri/src/services/mod.rs`.

### 3. Create the note folder in `note_create_empty` (in `HistoryController`)

After `self.history.append(&save_folder, record)`, create the folder:

```rust
let unix_ts = chrono::DateTime::parse_from_rfc3339(&record.created_at)
    .unwrap_or_default()
    .timestamp();
let folder_name = note_folder_name(unix_ts, &record.title);
let folder_path = std::path::Path::new(&save_folder).join(&folder_name);
std::fs::create_dir_all(&folder_path)?;
```

Store the folder path on the record by calling `self.history.update_session_dir(&save_folder, &record.id, &folder_path.to_string_lossy())`. Add `update_session_dir` to `HistoryService` following the `set_markdown_path` pattern — clone record, set `session_dir = Some(path.to_string())`, append, update cache.

### 4. Add `update_session_dir` to `HistoryService`

```rust
pub fn update_session_dir(&self, save_folder: &str, id: &str, dir: &str) -> Result<()>
```

Follows exact `set_markdown_path` pattern (lines 121–132 of `src-tauri/src/services/history.rs`).

### 5. Add `note_write_markdown` to `HistoryController`

```rust
/// Writes (or overwrites) `note.md` inside the note's session folder.
/// Only runs when `config.save_transcripts_as_markdown` is true.
pub fn write_note_markdown(&self, id: &str) -> Result<(), String>
```

Steps:
1. Get config: `let cfg = self.config.get();`
2. If `!cfg.save_transcripts_as_markdown` → return `Ok(())` early.
3. Get record: `self.history.get(&cfg.save_folder, id)?` — return `NotFound` if missing.
4. Build the markdown string (see format below).
5. Write to `{record.session_dir}/note.md` using `std::fs::write`.

**`note.md` format:**
```
---
title: '{title}'
created: '{created_at}'
tags: [{comma-separated tags}]
model: {model}
duration_seconds: {duration_ms / 1000.0}
word_count: {word_count}
---

## Notes

{written_content or empty}

## Transcript

{render_transcript_body output or empty}
```

For the transcript section, call `self.render_markdown(id)?` (existing method). If the record has no segments, emit an empty section body.

**When to call `write_note_markdown`:** from the controller methods that mutate note content:
- `update_written_content` → call `write_note_markdown` after the service update
- `attach_transcript` (story 0046) → call `write_note_markdown` after segments are updated
- `set_tags` / `set_layer_items` (story 0047) → call `write_note_markdown` after each

Do NOT add `write_note_markdown` to the IPC layer — it is an internal side effect of content mutations.

### 6. Tests

In `src-tauri/src/services/note_folder.rs` tests:

- `folder_name_contains_timestamp_and_words` — `note_folder_name(0, "Hello World Test")` starts with `"0100_01-01-70_Hello_World_Test_"` (unix 0 = 1970-01-01 01:00 UTC+1 or 00:00 UTC — pin to UTC for the test).
- `folder_name_caps_at_seven_words` — title with 10 words produces a folder name with exactly 7 word segments before the ID.
- `folder_name_id_is_six_chars` — the ID suffix after the last underscore is always 6 characters.
- `folder_name_strips_non_alpha` — `"Hello, World! 123 Test"` → words are `["Hello", "World", "Test"]`.

---

## Frontend

### 1. Update Settings label

In the settings UI (locate via `grep -rn "save_transcripts_as_markdown\|Save.*transcript" src/`), change the label string from `"Save transcripts as Markdown"` (or similar) to `"Save notes as Markdown"`. This is a one-line string change.

---

## SOLID / DRY expectations

- `note_folder_name` is a pure function in its own module — zero coupling to services or controllers.
- `write_note_markdown` is the single path for exporting `note.md` — it is called as a side effect from content-mutating controller methods, not duplicated in each command.
- Existing Scribe/Transcribe markdown export paths are untouched — `set_markdown_path` still handles those.

---

## Definition of done

- `cargo test -p scribefloat` passes (including `note_folder_name` tests)
- `cargo clippy -- -D warnings` passes
- Creating a new note creates a folder in `save_folder` with the correct naming pattern
- After typing content and autosaving, `note.md` in the folder contains the written content under `## Notes`
- After recording and Stop & Save, `note.md` includes the transcript under `## Transcript`
- Settings label reads "Save notes as Markdown"
- Old Scribe/Transcribe exports are unaffected
