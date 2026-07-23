# Issue tracker: Local Markdown

Issues and PRDs for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The PRD is `.scratch/<feature-slug>/PRD.md`
- Implementation issues are `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`
- Triage state is recorded as a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Wayfinder maps and decision tickets live under `.scratch/<effort-slug>/`.

### Layout

- **Map**: `.scratch/<effort-slug>/MAP.md` — the canonical index (`labels` include `wayfinder:map`)
- **Tickets**: `.scratch/<effort-slug>/issues/<NN>-<slug>.md` — child issues of that map
- **Known issues** (non-blocking debt dump for an effort): `.scratch/<effort-slug>/KNOWN-ISSUES.md` — disposable while the effort is live; moved wholesale into `docs/ideas/` (renamed, no curation) when the effort closes
- **Research findings**: linked from the ticket; usually `.scratch/<effort-slug>/research/<slug>.md`
- **Session bridge**: `.scratch/<effort-slug>/HANDOFF.md` — short-term working memory (what's true right now), distinct from `MAP.md`'s long-term memory (decisions so far). Every session in this effort updates it before ending, whether or not it resolved a ticket. See `docs/agents/working-method.md`.

### Ticket frontmatter

```yaml
---
title: <human-readable name — refer to tickets by this title>
labels: [wayfinder:grilling]   # wayfinder:map | wayfinder:research | wayfinder:prototype | wayfinder:grilling | wayfinder:task
status: open                   # open | closed
assignee:                      # empty = unclaimed; set before working a ticket
blocked_by: []                 # list of issue filenames this ticket waits on, e.g. ["03-adr-reality-audit.md"]
parent: MAP.md
---
```

### Operations

| Intent | How |
|---|---|
| Create map | Write `MAP.md` with Destination, Notes, Decisions so far, Not yet specified, Out of scope |
| Create child ticket | Add `issues/<NN>-<slug>.md` with `## Question`; set `parent: MAP.md` |
| Wire blocking | Second pass: fill each ticket's `blocked_by` with filenames of blockers |
| Claim | Set `assignee` before any work (e.g. agent id or `human`) |
| Frontier | Open tickets whose `blocked_by` are all `status: closed` and `assignee` is empty |
| Resolve | Append `## Resolution` (or a resolution comment under `## Comments`), set `status: closed`, append one gist line + link under the map's **Decisions so far** |
| Refer | Always use the ticket **title** in narration; path/id only inside the link |
