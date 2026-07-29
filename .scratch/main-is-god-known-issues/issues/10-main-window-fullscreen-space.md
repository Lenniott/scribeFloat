---
title: "Triage: Opening main window from tray lands on full-screen Space"
labels: [wayfinder:grilling]
status: open
assignee:
blocked_by: []
parent: ../MAP.md
---

## Issue

Opening the main window from the tray menu can switch the user into whatever Space a full-screen app currently occupies, instead of staying on the primary desktop — because the window-open path (`open_or_focus_window`/`raise_webview_window`) sets no macOS collection behavior at all and just relies on default AppKit activation heuristics. Same root-cause class as ticket 09.

## Question

Read the "Opening main window from tray lands on full-screen Space" entry in [docs/ideas/main-is-god-again-known-issues.md](../../../docs/ideas/main-is-god-again-known-issues.md) for full context. **Now** or **Later**? Mechanically small-medium (mirrors ticket 09's fix pattern), but the desired target behavior (always force primary desktop? just avoid full-screen Spaces?) is a product decision that needs to be made before implementing. Which behavior is wanted, and is it worth deciding now?

## Findings

**Where the tray-click / "open main window" handler lives:**
- `src-tauri/src/lib.rs:228-283` — `create_tray()`. The `TrayIconBuilder` is set with `.show_menu_on_left_click(true)` (line 233), so a tray click always shows the menu; `OPEN_APP_MENU_ID` (the "Open ScribeFloat" menu item, line 246-250) is what actually opens the main window, by calling `navigate_history_path(app, "")` (line 247).
- `navigate_history_path()` (`src-tauri/src/lib.rs:318-344`): if the `history` window already exists, it calls `raise_webview_window()` (line 321) and does `window.eval(...)` to update the route (line 327). If it doesn't exist, it builds it via `open_or_focus_window()` (lines 336-343) with plain defaults (no position/Space handling).
- `raise_webview_window()` (`src-tauri/src/lib.rs:460-480`): calls `.show()`, `.unminimize()`, `.set_focus()` — no Space or window-level handling of any kind.
- `open_or_focus_window()` (`src-tauri/src/lib.rs:482-...`): builds a fresh `WebviewWindowBuilder` with title/size only (line 497-499), no `.collection_behavior()`, no explicit positioning logic beyond Tauri defaults.

**Current behavior as coded:**
- There is no explicit Space or full-screen handling anywhere in the main-window-open path — it's just `.show()` / `.set_focus()` as noted in the ticket's own hypothesis. This is confirmed: repo-wide grep for `CollectionBehavior`, `NSWindow`, `WindowLevel`, `set_visible_on_all_workspaces`, `fullscreen`/`full_screen` window handling in `src-tauri/src` returns nothing relevant outside the Dictate `always_on_top` flag (a different window) and the Dock-icon objc shim in `platform/window_impl.rs`.
- Because the main `history` window is a normal, standard-collection-behavior NSWindow, macOS's own window-activation heuristics decide which Space to show/switch to when `.show()`/`.set_focus()` is called while a full-screen app is frontmost on another Space. This is default AppKit behavior (not a ScribeFloat bug per se) — apps that want a specific outcome (e.g. "always open on the primary desktop Space, never switch into someone else's full-screen Space") need to explicitly set collection behavior (e.g. `NSWindowCollectionBehaviorMoveToActiveSpace` vs. not, or handle activation policy) to override it.

**What a fix would touch:**
- Same class of fix as ticket 09: no builder-level Tauri API for this; would need a raw Cocoa call via `window.ns_window()` + objc `setCollectionBehavior:`/`level` in a new `platform::window_impl` helper, applied to the `history` window right after creation in `open_or_focus_window()` (`src-tauri/src/lib.rs:497` onward) and possibly reasserted in `raise_webview_window()` before `.show()` (line 461).
- The desired target behavior needs a product decision first (not just an engineering one): should opening from tray always force-switch to the primary desktop Space (`NSWindowCollectionBehaviorDefault`, explicitly not full-screen-auxiliary, possibly combined with deactivating/reactivating), or should it just avoid ever being window into the full-screen Space by using `NSApp.activateIgnoringOtherApps` plus explicit Space targeting? Different collection-behavior combinations produce different (sometimes surprising) results, so this will need on-device experimentation.

**Size estimate:** Small-to-medium for the mechanical code change (mirrors ticket 09's pattern), but the actual desired behavior is under-specified and macOS Space-switching semantics are finicky — expect iteration. Estimate: 0.5–1 day engineering + testing, assuming the desired behavior (e.g. "always land on primary desktop") is decided up front.
