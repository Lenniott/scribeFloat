---
id: "0050"
title: Note folder structure and markdown export
status: active
adr: ADR-0007
---

# Note folder structure and markdown export

Implement the new per-Note folder naming convention and update the auto-save-to-markdown feature to cover full notes (written source + transcript + YAML metadata frontmatter).

## What to build

**Folder name generation (Rust):**

```rust
fn note_folder_name(created_at: f64, title: &str) -> String {
    // 1. Extract up to 7 alphabetical words from title (split on non-alpha)
    // 2. Format: HHMM_DD-MM-YY_word1_word2_..._XXXXXX
    // 3. ID: MD5("{created_at} {7-word-title}") → base-36, 6 chars, uppercase
}
```

Port the Python reference implementation from ADR-0007 to Rust. Use the `md5` crate (or `sha2` already present — use MD5 via a small inline impl or add `md5 = "0.10"` crate).

**Note folder creation:** `note_create_empty` (story 0049) creates the folder at `{save_folder}/{note_folder_name}/` immediately on Note creation.

**Audio retention:** Audio files streamed during recording (story 0046) go directly into the Note folder. They are never deleted after processing (no `keep_wavs` toggle for this flow).

**Markdown export (`note.md`):** When `save_transcripts_as_markdown` is enabled (existing setting, widened to cover notes), write `{note_folder}/note.md` on:
- Every autosave of written content (debounced with the content autosave)
- Transcript attachment (after DONE event, story 0046)
- Metadata save

Format:
```markdown
---
title: 'My Note Title'
created: '2025-06-04T10:30:00'
tags: [tag1, tag2]
keywords: [kw1, kw2]
model: base
duration_seconds: 142.0
word_count: 847
---

## Notes

{written source content}

## Transcript

{rendered transcript body}
```

## Notes

- Existing notes (pre-this-story) keep their current flat file structure; migration is out of scope
- The `save_transcripts_as_markdown` setting label in Settings → General should be updated to "Save notes as markdown" 
- Title capping to 7 words is for the **folder name only**; the display title is uncapped
- Depends on 0049 (note_create_empty, which is where the folder is created) and 0046 (transcript attachment)
