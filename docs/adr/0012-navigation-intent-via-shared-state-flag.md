# ADR-0012: Navigation intent via shared app state flag

## Status

Accepted

## Context

SvelteKit navigates by URL — a component that needs to perform a side-effect immediately on mount (e.g. auto-start recording) cannot receive instructions through the route itself without polluting the URL or adding a route parameter. The problem arose when the TitleBar "Record" button needed to navigate to `/notes/new` and have the `RecordingStrip` component auto-start recording as soon as the new note page mounted.

Passing data via `goto` search params would expose internal state in the URL and require the receiving component to clean up the URL after reading. Using a Tauri-layer event bus would add cross-layer coupling for a purely frontend concern. A Svelte context or prop cannot survive a navigation boundary.

## Decision

We will use a short-lived boolean flag on the singleton `appState` store (`scribeAutoStart`) as a navigation-intent signal. The sender sets the flag immediately before calling `goto()`; the receiver reads and clears it in `onMount`. The flag is consumed exactly once and is reset to `false` whether or not it was acted on.

## Consequences

- Simple to implement and reason about — no new abstractions.
- The flag is only meaningful during the narrow window between `goto()` and the next component's `onMount`. Any navigation that bypasses `onMount` (e.g. back/forward cache hit) will silently ignore the flag — acceptable because the auto-start path always creates a fresh route.
- The pattern is limited to fire-once intent signals where losing the signal on a missed mount is acceptable. It must not be used for persistent state or multi-consumer signals.
- Future callers that need similar cross-navigation intent (e.g. auto-focus a field, pre-populate a form) should follow this same pattern rather than inventing alternatives.
