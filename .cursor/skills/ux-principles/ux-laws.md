---
type: reference
topic: ux-laws
when: Something feels harder to use than it should and you need to diagnose why. Or you're writing a design justification.
see-also:
  - principles.md — the ScribeFloat principles that apply these laws
---

# UX Laws Cheat Sheet

> **When to read this**: Something feels harder to use than it should and you need to work out why. Or you want the science behind a design decision.

You do not need to memorise these. Use them when reviewing a design and need to name what's wrong, or when justifying a decision to a collaborator.

---

## Interaction & Input

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Fitts's Law** | Time to reach a target depends on distance and size | Make primary actions large and position them where users already are |
| **Hick's Law** | Decision time increases with the number of choices | Reduce options, use defaults, break complex tasks into steps |
| **Doherty Threshold** | People get more done when the system responds in under 400ms | Provide instant feedback: skeletons, optimistic UI, loading indicators |
| **Postel's Law** | Be liberal in what you accept, conservative in what you send | Accept varied input formats, output clean consistent results |

---

## Perception & Grouping

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Proximity** | Nearby elements are perceived as related | Group related controls with whitespace between unrelated groups |
| **Similarity** | Similar-looking elements feel like they belong together | Consistent styling for same-function elements |
| **Common Region** | A shared boundary groups elements | Use cards, borders, backgrounds to visually group content |
| **Pragnanz** | People simplify complex visuals | If a design needs explanation, simplify it |

---

## Memory & Cognition

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Miller's Law** | Working memory holds around 7 items | Chunk long lists, limit tabs, break up large forms |
| **Cognitive Load** | Mental effort adds up quickly | Reduce decisions, use familiar patterns, carry data forward |
| **Jakob's Law** | Users expect your product to work like tools they already know | Reuse patterns from Notion, Slack, and native macOS apps where they fit |
| **Mental Model** | Users bring expectations from other tools | Match existing patterns before inventing new ones |

---

## Attention & Recall

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Von Restorff Effect** | The different item is remembered | Use visual emphasis sparingly — if everything stands out, nothing does |
| **Serial Position Effect** | First and last items are best remembered | Put important actions at the start and end of lists and toolbars |
| **Zeigarnik Effect** | Incomplete tasks are remembered | Use progress indicators and checklists to drive completion |

---

## Decision Making

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Choice Overload** | Too many options slow people down | Limit visible options, use smart defaults, offer curation |
| **Occam's Razor** | The simplest solution is usually the best | Between two designs, choose fewer elements, fewer steps |
| **Paradox of the Active User** | Most users will not read instructions | Make interfaces self-explanatory with inline hints and progressive disclosure |

---

## Experience & Motivation

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Aesthetic-Usability Effect** | Better-looking interfaces often feel easier to use | Visual polish builds trust and forgiveness for minor issues |
| **Peak-End Rule** | Experiences are judged by peak moments and the ending | Polish key moments: first use, task completion, error recovery |
| **Goal-Gradient Effect** | Motivation increases near the goal | Show progress bars and step indicators in multi-step flows |
| **Flow** | People work best when they are not being interrupted | Minimise interruptions, eliminate unnecessary confirmations |

---

## Complexity & Effort

| Law | Summary | ScribeFloat takeaway |
|---|---|---|
| **Tesler's Law** | Complexity cannot be removed, only shifted | Absorb complexity on the engineering side — see principle #10 |
| **Pareto Principle** | 80% of usage comes from 20% of features | Optimise ruthlessly for the core recording → transcript → review path |
| **Parkinson's Law** | Tasks expand to fill available time | Set expectations with deadlines, limits, and progress indicators |

---

→ **Laws in context**: `principles.md` explains how each maps to ScribeFloat's 10 core principles
