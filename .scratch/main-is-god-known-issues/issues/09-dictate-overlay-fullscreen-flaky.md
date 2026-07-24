---
title: "Triage: Dictate overlay flaky in macOS full-screen / other Spaces"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Question

Read the "Dictate overlay flaky in macOS full-screen / other Spaces" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) (search for that heading). Decide: is this **Now** (fix it in this effort — state the concrete change, then make it) or **Later** (state why, and whether it's big enough to need its own future wayfinder)?

## Findings

**Where the Dictate overlay window is created:**
- `src-tauri/src/lib.rs:287-309` — `prewarm_dictate_window()`: builds the `dictate` window at startup (hidden), via `WebviewWindowBuilder`.
- `src-tauri/src/lib.rs:415-439` — `open_dictate_window()`: reuses the prewarmed window if present (repositions + `.show()`, line 419) or builds it fresh (lines 424-437) when triggered.
- Both builders set: `.inner_size(...)`, `.decorations(false)`, `.resizable(false)`, `.always_on_top(true)`, `.skip_taskbar(true)`, `.shadow(true)`, `.position(x, y)`, `.visible(...)`.

**Current behavior as coded:**
- No `collectionBehavior` is set anywhere. Tauri's `WebviewWindowBuilder` on macOS has no builder method for `NSWindowCollectionBehavior` at all (as of the Tauri version used here) — there is no `.collection_behavior(...)` call in this codebase, and a repo-wide grep for `CollectionBehavior`, `NSWindow`, `set_visible_on_all_workspaces`, `WindowLevel`, `joins_full_screen`, `can_join_all_spaces` turns up **zero** hits in `src-tauri/src` outside `platform/window_impl.rs`, which only handles the Dock icon (raw `objc_msgSend` calls to `NSApplication`/`NSImage`, lines 59-124) — nothing window-level.
- `always_on_top(true)` only affects window layering (`NSWindowLevel`) within whatever Space the window is on; it does not opt the window into `NSWindowCollectionBehaviorFullScreenAuxiliary` / `.canJoinAllSpaces`. By default, an NSWindow's collection behavior is `NSWindowCollectionBehaviorDefault`, meaning it belongs only to the Space it was created on and does **not** float above a full-screen Space. This matches the reported symptom: capture (audio/hotkey path, independent of window visibility) keeps working, but the HUD is invisible when a full-screen app is frontmost on another Space, because macOS simply won't display a default-behavior window over a full-screen Space.
- The "sometimes follows back to the main desktop" behavior is consistent with `open_dictate_window()` repositioning + showing the existing window (line 417-420): if the window was left on the primary desktop's Space from a previous show, macOS may switch Spaces to reveal it, or leave it stranded depending on which Space was active at build vs. show time — there's no explicit Space affinity, so behavior is at the mercy of macOS's default heuristics.

**What a fix would touch:**
- Tauri's Rust API does not expose collection behavior directly (no `.collection_behavior()` builder method as of the `tauri` crate version pinned in `src-tauri/Cargo.toml` — verify exact version before implementing). The fix requires dropping to raw Cocoa/objc, similar to the existing pattern in `platform/window_impl.rs`'s `mod macos` block (lines 59-124): obtain the `NSWindow*` via `window.ns_window()` (Tauri exposes this raw pointer accessor on `WebviewWindow` for macOS), then call `setCollectionBehavior:` with `NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary` (and typically also bump `NSWindowLevel` to `NSStatusWindowLevel` or `NSPopUpMenuWindowLevel` so it draws above the full-screen app's own chrome).
- This would be a new function, e.g. `platform::window_impl::set_dictate_collection_behavior(&window)`, called once right after `.build()` in both `prewarm_dictate_window()` (line 303) and `open_dictate_window()` (line 437), guarded by `#[cfg(target_os = "macos")]`.
- Needs manual testing on real full-screen Spaces (simulators/CI can't easily exercise this) since behavior varies between "full-screen app" and "Split View" Spaces.

**Size estimate:** Small-to-medium. The raw objc pattern already exists in the codebase to copy from, but full-screen Space behavior is notoriously fiddly on macOS (collection behavior + window level combinations, activation policy interactions) and will likely need a few iterations of manual, on-device testing across macOS versions to get right. Estimate: 0.5–1.5 days including testing.
