---
name: new-adr
description: >-
  Create a new Architecture Decision Record in docs/adr/. Use when the user
  wants to record a decision, says "write an ADR", "capture this decision",
  or "ADR for X". Always creates a file and updates the ADR index.
---

# /new-adr

Create an ADR file in `docs/adr/`. One file per decision, named `NNNN-slug.md`.

## Steps

**1. Determine the next ADR number**

```bash
ls docs/adr/ | grep -oE '^[0-9]{4}' | sort -n | tail -1
```

Increment by 1. Zero-pad to 4 digits.

**2. Derive the slug**

Kebab-case summary of the decision. Max 6 words.

**3. Gather the four fields** (from conversation or ask)

- **Context** — what situation or problem forced this decision?
- **Decision** — the specific choice made, stated plainly
- **Consequences** — what gets easier, what gets harder, what is now off the table

Title is inferred from the decision if not given explicitly.

**4. Write the file**

```
docs/adr/NNNN-slug.md
```

**5. Update the ADR index** at `docs/adr/README.md` (create it if missing)

Add one line: `| ADR-NNNN | [slug](NNNN-slug.md) | one-line summary |`

**6. Confirm**

Tell the user the ADR number and filename.

## Template

```markdown
# ADR-NNNN: Title

## Status

Accepted

## Context

Why this decision was needed — the forces, constraints, or problem being solved.

## Decision

The specific choice. Stated as a fact, not a proposal. "We will..." or "X owns Y."

## Consequences

What becomes easier or more constrained as a result. What is now explicitly out of scope.
```

## Rules

- Status is always `Accepted` unless the user says otherwise (`Proposed`, `Superseded`)
- Decisions are permanent record — do not soften or hedge the decision statement
- Always update `docs/adr/README.md` index after writing the file
- If the user gives a decision title, use it verbatim
