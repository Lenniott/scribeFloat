# Broccoli Memory Experiment Package

This folder packages the local-first conversation memory experiment for transfer into the main app project.

The goal is not to ship this code directly. The goal is to preserve the product learning:

> Users should be able to ask for a context pack from existing conversation memory without reading the transcripts first.

## Contents

```text
docs/
  AGENT_CONTEXT.md          Context for an agent picking this up in the app project
  PRODUCT_LEARNINGS.md      Product decisions and lessons from the experiment
  INTEGRATION_NOTES.md      Suggested app integration shape

prototype/
  run_broccoli.py           Local prototype script
  config.json               Ollama/model config used for the test
  requirements.txt          Python dependencies

pack_briefs/
  sedgwick_workflow_automation_opportunities.json

example_outputs/
  sedgwick_workflow_automation_opportunities.md
  sedgwick_workflow_automation_opportunities.json
  index_summary.json
```

Raw transcripts are intentionally not included in this package. The package is meant to carry the experiment context and app-facing learning, not private source data.

## Core Workflow

1. Day 1: transcripts are processed into general, source-linked thread memories.
2. Day 2: the user gives a lightweight pack brief.
3. The system expands that brief into retrieval signals.
4. The system retrieves relevant stored memories.
5. The system writes a source-linked context pack.

Core rule:

> Memories are general. Context packs are contextual.

