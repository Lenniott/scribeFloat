---
id: "0037"
title: Build app-shell.svelte — persistent sidebar shell replacing +page.svelte
status: done
exploration: design-brain-prd.md
---

# Build app-shell.svelte

Replace the current `+page.svelte` routing with a persistent sidebar shell. The shell owns all global state: note list, toast, delete confirmation modal, and navigation route.

## Acceptance criteria

- Sidebar Region always visible (except when Settings sidebar swaps in)
- Routes: `home | notes | upload | float | settings`
- `float` renders a placeholder "Coming soon" Area
- IPC navigation event: `app://navigate` (not `shell://navigate`)
- Global state lives in the shell, not in child screens
- Leave guard for Capture screen wired through shell

## Reference

Exploration code: `src/lib/screens/app-shell.svelte` on `release/0.3`.
All naming must follow CONTEXT.md — rename any `dashboard`, `transcripts`, `shell` terms.
