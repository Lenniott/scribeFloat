---
title: Remove Float coming-soon from shipped UI
labels: [wayfinder:task]
status: closed
assignee: cursor-agent
blocked_by: []
parent: MAP.md
---

## Question

Where does the shipped app still promise Float / “coming soon” Float, and what minimal UI cut removes that false promise without a Float funeral in code/docs?

**Done when:** Inventory + decided cut recorded; false promise gone from user-facing chrome (execution is in scope for this task ticket’s resolution).

## Inventory

| Surface | Was | Cut |
|---------|-----|-----|
| Sidebar `Float` + “Coming soon” | Shipped nav tease | Removed nav item; dropped `float` from `AppRoute` |
| `/float` page | “Float — coming soon” | Route deleted |
| Home stats | “Float layers” + “Drafts to review” both always `—` | Removed both empty promise tiles; kept Notes + Recorded this week |
| Note editor metadata pane | “Float layers — story 0047” | Neutral “not wired yet” |
| FilterPanel empty copy | “approve a Float result…” | Neutral empty state |
| Design-system NavItem demo | Float / Coming soon | Demo label only (`Reports` / Unavailable) |
| `CONTEXT.md` Float glossary + `float_layers` field | Domain / schema | **Left** — not a UI tease; funeral later |

## Resolution

Minimal UI honesty cut applied. No Float code/docs funeral. Domain term “Float” remains in glossary for later maps.

## Comments

- 2026-07-19: claimed + resolved by cursor-agent in morning handoff session.
