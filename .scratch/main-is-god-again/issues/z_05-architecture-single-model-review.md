---
title: Architecture and single-model review
labels: [wayfinder:research]
status: closed
assignee: research-agent
blocked_by: []
parent: MAP.md
---

## Question

Against a written rubric (seams/testability, capture pipeline consistency, Notes/speakers model, and especially **one model** truth in UI + code), what architecture findings exist on the current spine?

Must include an inventory of remaining multi-model download/chooser UI and dead code paths. Each finding: evidence, and suggested **merge-blocker** vs **Known issues**. Dead multi-model paths are expected merge-blockers per map decisions — list them concretely for deletion.

## Resolution

Findings written: [architecture-single-model-review.md](../research/architecture-single-model-review.md).

**Gist:** In-app multi-model download/chooser UI is already gone (`cef8c57`); merge is still blocked by (1) dead selection machinery (`selected_model_id` / `scribe_model_path` / catalog-id resolvers / Upload `modelId`) and (2) product/docs that still sell Settings → Models and fast vs refined Whisper. Inventory table in the research doc feeds *Delete dead multi-model paths*.
