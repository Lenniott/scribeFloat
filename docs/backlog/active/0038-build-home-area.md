---
id: "0038"
title: Build Home Area screen
status: active
---

# Build Home Area (home.svelte)

The Home Area is the landing screen. Shows stat tiles and recent Notes.

## Acceptance criteria

- File: `src/lib/screens/home.svelte`
- Components: `StatTile` (primitive), `RecentNoteCard` (component)
- Stat tiles: Transcript count, Float layers (placeholder —), Drafts to review (placeholder —), Recorded this week
- Recent Notes: last 6, clicking navigates to Notes detail
- Listens to `history://item-added` to refresh stats
- "See all →" navigates to Notes Area

## Reference

Exploration: `src/lib/screens/dashboard.svelte` on `release/0.3`.
Rename: `Dashboard` → `Home`, `RecentSessionCard` → `RecentNoteCard`.
