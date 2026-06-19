---
id: "0046"
title: Move cleanup_text regexes to LazyLock statics
status: active
priority: medium
---

# Move `cleanup_text` regexes to `LazyLock` statics

## Problem

`cleanup_text` in `src-tauri/src/services/output.rs` (around line 657) compiles three `Regex` objects on **every call**:

```rust
fn cleanup_text(text: &str) -> String {
    let caps_re   = Regex::new(r"\[[A-Z][A-Za-z_ ]*\]").expect("static regex");
    let noise_re  = Regex::new(r"(?i)\[(silence|noise|music|applause|laughter|inaudible)\]").expect("static regex");
    let fusion_re = Regex::new(r"(?i)(#\w+?)(newline)").expect("static regex");
    // ... apply regexes
}
```

`cleanup_text` is called **once per transcription segment**. A 60-minute recording at Whisper's 30-second chunk rate produces ~120 segments, meaning ~360 unnecessary regex compilations per session. Regex compilation is not free — it allocates and runs a DFA construction algorithm.

This violates DRY (the pattern strings are de-facto constants repeated in code flow) and wastes CPU in the transcription hot-path.

## Fix

Lift all three patterns to `std::sync::LazyLock<Regex>` module-level statics. `LazyLock` is stable since Rust 1.80 and is the idiomatic replacement for `once_cell::sync::Lazy` (no extra dependency needed).

```rust
use std::sync::LazyLock;
use regex::Regex;

static CAPS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[[A-Z][A-Za-z_ ]*\]").expect("static regex")
});

static NOISE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\[(silence|noise|music|applause|laughter|inaudible)\]").expect("static regex")
});

static FUSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(#\w+?)(newline)").expect("static regex")
});

fn cleanup_text(text: &str) -> String {
    // Use &*CAPS_RE, &*NOISE_RE, &*FUSION_RE — same logic, zero recompilation
}
```

## Scope

- Touch only `src-tauri/src/services/output.rs`.
- Do not change the function signature or observable behaviour of `cleanup_text`.
- Scan the rest of `output.rs` for any other `Regex::new(...)` calls inside functions and hoist those too if found.

## Acceptance criteria

- [ ] No `Regex::new` call appears inside any function body in `output.rs`.
- [ ] All regex patterns are `LazyLock<Regex>` statics at module scope.
- [ ] `cleanup_text` produces identical output for any given input before and after the change.
- [ ] Existing tests in `output.rs` `#[cfg(test)]` pass without modification.
- [ ] `cargo test -p scribefloat` passes.
- [ ] `cargo clippy -- -D warnings` passes.

## Notes

- `LazyLock` requires Rust ≥ 1.80. Check `rust-toolchain.toml` or `Cargo.toml` `rust-edition` to confirm the MSRV. If below 1.80, use `once_cell::sync::Lazy` from the already-present `once_cell` crate instead (check `Cargo.toml`).
- `&*STATIC_NAME` dereferences the `LazyLock` to get `&Regex` — this is the standard usage pattern.
- The `.expect("static regex")` calls in the statics are acceptable: if a hardcoded literal fails to compile, it is a programming error that should panic at first use, not be silently swallowed.
