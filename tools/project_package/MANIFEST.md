# Package Manifest

Package created for the main app project.

## Purpose

Give an agent enough context to integrate the experiment learning into the app without needing this conversation thread or the raw transcripts.

## Include In App Project

Recommended files to copy into the app project:

```text
project_package/README.md
project_package/docs/AGENT_CONTEXT.md
project_package/docs/PRODUCT_LEARNINGS.md
project_package/docs/INTEGRATION_NOTES.md
project_package/pack_briefs/PACK_BRIEF_SCHEMA.md
project_package/pack_briefs/sedgwick_workflow_automation_opportunities.json
project_package/example_outputs/sedgwick_workflow_automation_opportunities.md
project_package/example_outputs/sedgwick_workflow_automation_opportunities.json
```

The `prototype/` folder is optional. Keep it if the app project agent needs runnable reference code.

## Excluded

Raw transcript files are excluded by design.

Reason: the package is for product and implementation learning, not for moving private test data around.

## Most Important Takeaway

The app should let the user create a context pack from a lightweight, transcript-agnostic brief. The user supplies intent; the system expands, retrieves, gates relevance, and assembles source-linked context.

