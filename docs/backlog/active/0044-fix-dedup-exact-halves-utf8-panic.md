---
id: "0044"
title: Fix UTF-8 panic in dedup_exact_halves
status: active
priority: high
---

# Fix UTF-8 panic in `dedup_exact_halves`

## Problem

`dedup_exact_halves` in `src-tauri/src/services/output.rs` (around line 718) splits a string at a **byte** midpoint rather than a **character** boundary. On any transcript that contains a multi-byte UTF-8 character (accented letters, emoji, curly quotes, non-Latin scripts) where that character straddles the midpoint, `str::split_at` panics with `byte index N is not a char boundary`.

Buggy code (simplified):

```rust
fn dedup_exact_halves(text: &str) -> String {
    let trimmed = text.trim();
    let mid = trimmed.len() / 2;          // byte offset — WRONG for multi-byte chars
    let (first, second) = trimmed.split_at(mid);  // panics if mid is inside a char
    if first.trim() == second.trim() {
        return first.trim().to_string();
    }
    text.to_string()
}
```

This is a **correctness bug** (silent data loss path + potential panic in production). It is classified HIGH because:
- Whisper outputs are frequently non-ASCII (filler sounds rendered as `[MUSIC]`, accented proper nouns, etc.).
- The panic is unrecoverable — it crashes the transcription thread.

## Fix

Replace the byte-offset midpoint with a **character-boundary** midpoint using `char_indices`.

```rust
fn dedup_exact_halves(text: &str) -> String {
    let trimmed = text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 {
        return text.to_string();
    }
    // Walk to the character at position char_count/2 to get a valid byte offset.
    let mid_byte = trimmed
        .char_indices()
        .nth(char_count / 2)
        .map(|(i, _)| i)
        .unwrap_or(trimmed.len());
    let first = &trimmed[..mid_byte];
    let second = &trimmed[mid_byte..];
    if first.trim() == second.trim() {
        first.trim().to_string()
    } else {
        text.to_string()
    }
}
```

Key properties:
- `char_indices().nth(n)` always lands on a valid char boundary — no panic possible.
- Odd character counts round down (`char_count / 2`), matching original intent.
- Zero-length strings handled explicitly before indexing.

## Acceptance criteria

- [ ] `dedup_exact_halves` does not panic on any valid UTF-8 input.
- [ ] `dedup_exact_halves("café café")` returns `"café"` (accent on `e`).
- [ ] `dedup_exact_halves("hello hello")` returns `"hello"` (ASCII, regression guard).
- [ ] `dedup_exact_halves("hello world")` returns `"hello world"` (no dedup — different halves).
- [ ] `dedup_exact_halves("")` returns `""`.
- [ ] Unit tests cover all four cases above and live inside `src-tauri/src/services/output.rs` `#[cfg(test)]` block.
- [ ] `cargo test -p scribefloat` passes.
- [ ] `cargo clippy -- -D warnings` passes.

## Notes

- Do not change the function signature or its call sites — it is a private helper called only within `output.rs`.
- Do not add `unsafe`. The safe `char_indices` approach has zero overhead for typical transcript lengths.
- The test for `"café café"` requires a string where the accent character falls at or near the split point — verify the byte layout manually if needed.
