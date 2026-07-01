# Product Learnings

## Main Learning

The context-pack feature should be designed around a lightweight brief, not a recipe.

The user should be able to say roughly what they need, add a few broad tags or signals if useful, and let the system search the stored memory layer.

## Desired User Burden

Acceptable:

- one sentence describing the pack need
- optional broad tags
- optional broad signals

Not acceptable:

- asking the user to read transcripts first
- asking the user to provide detailed transcript-specific keywords
- asking the user to choose memories manually before the system can help
- forcing a fixed pack recipe system

## Memory Model

The memory layer should stay general:

- thread ID
- line of inquiry
- summary
- status
- confidence
- source-linked occurrences
- aspect counts

Occurrences should carry categories, but categories should not become the main product object.

## Pack Model

A context pack should be contextual:

- user brief
- expanded retrieval brief
- selected memories
- workflow/use-case leads
- source excerpts and source references
- caveats when interpretation is provisional
- clear empty state when no relevant memories exist

## Retrieval Lesson

Embedding similarity alone is not enough. The first AI-role pack retrieved unrelated memories because it always returned the top matches.

The retrieval system needs some form of relevance gate:

- matched distinctive request terms
- topic/tag overlap
- reranking
- LLM relevance check
- or another "is this actually about the request?" step

No result is better than a plausible but unrelated pack.

## App Implication

The app should store memories in a way that supports later contextual retrieval:

- every memory should be source-linked
- every occurrence should keep enough source text to audit
- embeddings should be attached to memory objects
- pack generation should store the expanded brief and selected memory IDs

