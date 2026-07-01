# Integration Notes

## Suggested App Shape

Add three concepts to the app:

1. Stored memory
2. Pack brief
3. Context pack

## Stored Memory

Created during transcript processing. It should not assume a future use case.

Minimum useful fields:

```json
{
  "memory_id": "mem_x",
  "thread_id": "thread_001",
  "line_of_inquiry": "...",
  "summary": "...",
  "status": "open",
  "confidence": "medium",
  "occurrences": [
    {
      "source_id": "...",
      "unit_start": 10,
      "unit_end": 18,
      "category": "Problem",
      "summary": "...",
      "source_excerpt": "..."
    }
  ],
  "embedding_text": "..."
}
```

## Pack Brief

Created later by the user. It should be deliberately lightweight.

```json
{
  "pack_id": "workflow_automation_opportunities",
  "request": "What workflows and problems could light automation help with?",
  "tags": ["workflow_discovery", "light_automation"],
  "signals": ["tax", "planning", "forms", "reports"]
}
```

The app should expand this brief internally. The expanded brief should be saved for debugging and product learning.

## Context Pack

Generated from stored memories.

Minimum useful fields:

```json
{
  "pack_id": "workflow_automation_opportunities",
  "request": {},
  "expanded_brief": {},
  "selected_memory_ids": [],
  "created_at": "..."
}
```

The readable Markdown or UI view should start with the synthesized leads, then show source memories underneath.

## Empty State

If the stored memories do not support the pack request, the app should say that directly:

> No sufficiently relevant memories found in the processed transcripts.

Do not force unrelated nearest-neighbor matches into a pack.

## Next Experiment

The next useful experiment is not schema work. It is quality work:

1. improve structured memory extraction
2. improve pack relevance gating
3. add a synthesis/rerank pass over retrieved memories
4. compare whether lightweight briefs produce useful packs across several user needs

