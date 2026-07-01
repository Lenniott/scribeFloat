# Lightweight Pack Brief Schema

This is the intended product-facing shape.

```json
{
  "pack_id": "optional_stable_id",
  "title": "Optional title",
  "request": "One sentence describing the context the user needs.",
  "tags": ["optional", "broad", "tags"],
  "signals": ["optional", "broad", "retrieval", "signals"],
  "max_memories": 10,
  "max_occurrences_per_memory": 4
}
```

The user should only need to provide `request`. Tags and signals are optional helpers.

The system should expand the brief into retrieval language and save that expanded brief beside the generated pack.

