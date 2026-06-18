---
name: new-story
description: >-
  Create a new backlog story file in docs/backlog/active/. Use when the user
  wants to capture a piece of work, says "add a story", "create a story",
  "backlog this", or "write a story for X". Always creates a file — never
  just describes what the story would contain.
---

# /new-story

Create a story file in `docs/backlog/active/`. One file per story, named `NNNN-slug.md`.

## Steps

**1. Determine the next ID**

```bash
ls docs/backlog/active/ docs/backlog/archived/ | grep -oE '^[0-9]{4}' | sort -n | tail -1
```

Increment by 1. Zero-pad to 4 digits.

**2. Derive the slug**

Kebab-case from the title. Max 5 words. Example: `rename-historyrecord-to-note`.

**3. Gather fields** (from user message or ask if missing)

- `title` — one short imperative sentence
- `adr` — optional, e.g. `ADR-0001`
- `exploration` — optional, filename only, e.g. `2026-06-18-tooling-and-doc-system-design.md`

**4. Write the file**

```
docs/backlog/active/NNNN-slug.md
```

Frontmatter + a body with enough context for an agent to pick it up cold. Do not pad — a two-line body is fine if the title is self-explanatory.

**5. Confirm**

Tell the user the filename. No other output needed.

## Template

```markdown
---
id: "NNNN"
title: Title here
status: active
adr: ADR-NNNN        # delete if not applicable
exploration: filename  # delete if not applicable
---

# Title here

One paragraph: what needs to be done and why, with enough context for an agent to start without re-reading the conversation.

## Notes (optional)

Any constraints, dependencies on other stories, or things not to do.
```

## Rules

- `status` is always `active` on creation
- Never create the file without writing it — do not just print the content
- If the user gives a title, use it verbatim; do not reword it
- If multiple stories are needed, create all of them in one turn
