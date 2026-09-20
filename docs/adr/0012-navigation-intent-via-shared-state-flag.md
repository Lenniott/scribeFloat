# ADR-0012: Navigation intent via shared app state flag

**Status:** Binding (pattern). Original instance removed.
**Wayfinder:** Pattern from Main is God again. The `appState.scribeAutoStart` flag was removed 2026-07-29 (ticket 11, record-button context). TitleBar **"New note"** creates a note and does **not** auto-start recording; **"Record"** appears only on an open note and starts capture there. Do not restore auto-record-on-new-note.

## Context

SvelteKit navigates by URL — a component that needs to perform a side-effect immediately on mount cannot receive instructions through the route itself without polluting the URL or adding a route parameter. The original case was the TitleBar button navigating to `/notes/new` and auto-starting recording on mount.

Passing data via `goto` search params would expose internal state in the URL and require the receiving component to clean up the URL after reading. Using a Tauri-layer event bus would add cross-layer coupling for a purely frontend concern. A Svelte context or prop cannot survive a navigation boundary.

That auto-start product behaviour was later reversed so TitleBar matches the tray: "New note" only creates; recording starts from "Record" on an already-open note. The flag became dead code and was deleted. The *pattern* for fire-once cross-navigation intent remains.

## Decision

Cross-navigation, fire-once intent uses a short-lived field on the singleton `appState` store. The sender sets it immediately before `goto()`; the receiver reads and clears it in `onMount`. The flag is consumed exactly once and is reset whether or not it was acted on.

The original field was `scribeAutoStart`. **That field no longer exists.** Current TitleBar behaviour does not need a navigation-intent flag for Record.

## Consequences

- Simple to implement and reason about — no new abstractions.
- The flag is only meaningful during the narrow window between `goto()` and the next component's `onMount`. Any navigation that bypasses `onMount` (e.g. back/forward cache hit) will silently ignore the flag — acceptable for fire-once intent on a fresh route.
- The pattern is limited to fire-once intent signals where losing the signal on a missed mount is acceptable. It must not be used for persistent state or multi-consumer signals.
- Future callers that need similar cross-navigation intent (e.g. auto-focus a field, pre-populate a form) should follow this same pattern rather than inventing alternatives.
- Do not reintroduce `scribeAutoStart` or auto-start recording when creating a note from the title bar or tray "New note".
