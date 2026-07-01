# Agent Context

You are picking up a local-first experiment for a ScribeFloat-style conversation memory feature.

The experiment tested whether messy transcripts can be processed into durable memory objects, then later searched to create source-linked context packs from a lightweight user brief.

## Current Product Direction

The desired product behavior is a two-stage workflow:

1. Record/process now.
2. Ask for context later.

Processing should not assume a future use case. A transcript should become general memory.

The user should later be able to define a context pack without reading the transcripts first. The pack brief should be lightweight:

```json
{
  "request": "What everyday Sedgwick White workflows and problems could light automation or AI-assisted workflows help with?",
  "tags": ["workflow_discovery", "light_automation", "client_service"],
  "signals": ["tax", "planning", "forms", "reports", "documents", "marketing"]
}
```

The system, not the user, should do the work of expanding that brief into retrieval signals and finding relevant memories.

## Important Lessons

- Do not make users define fixed recipes.
- Do not require users to know transcript contents.
- Do not make ingestion role-specific.
- Do not over-optimize the prototype into a clean architecture too early.
- The useful product object is closer to a thread memory than a flat fragment list.
- Thread titles are optional; IDs, summaries, lines of inquiry, source occurrences, and categories may be enough.
- The seven categories are useful as aspects on source occurrences:
  - Situation
  - Problem
  - Intent
  - Option
  - Decision
  - Evidence
  - Open Thread

## What Went Wrong

The first attempted pack asked for an AI-role context pack. The transcripts were not actually about that. The retrieval system forced nearest embedding matches and produced unrelated material.

The fix was to treat pack retrieval as relevance-gated, not just nearest-neighbor ranking. If no relevant memories are found, the pack should say so.

## What Improved

The later pack brief targeted ordinary Sedgwick White workflows and problems hiding inside AI-heavy ideation conversations. That produced a more useful direction:

- tax return work feeding planning advice
- website, SEO, and marketing content workflow
- risk assessment and report production
- client data, folders, and document routing
- fact-finding forms and client intake

## Current Caveat

The local model returned poor structured JSON during the experiment, so fallback extraction was used. Treat the outputs as product-learning artifacts, not high-quality model evaluations.

