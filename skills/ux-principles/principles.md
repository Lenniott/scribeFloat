---
type: rules
topic: principles
when: Justifying a design decision; understanding why a pattern rule exists; reviewing work against first principles.
see-also:
  - user-context.md — who these principles serve
  - ux-laws.md — the UX science behind each principle
---

# Core Principles

> **When to read this**: You want to understand the *why* behind a design decision, or you need to justify a choice in a review.

Ten principles, in priority order. ScribeFloat raises the bar on how tightly each must be applied — these are not aspirational values, they are the basis for every interaction rule in this tree.

---

## 1. Respect the user's time

Every extra click, unnecessary animation, and vague label costs someone time. The best interfaces stay out of the way.

*In ScribeFloat*: Users return to this tool dozens of times per week for fast, repeatable tasks. Friction compounds. Optimise for zero-hesitation on the main path.

→ Connects to: `interaction.md` (immediate feedback, no duplicate requests), `motion.md` (skip unnecessary animation)

---

## 2. Reduce cognitive load

People have limited working memory. Do not make them hold information between steps, weigh 20 options when 5 will do, or make decisions the system could make for them.

*In ScribeFloat*: Knowledge workers are already processing conversation content. The tool should demand as little of their attention as possible.

→ Connects to: `interaction.md` (forms + filters), `data-display.md` (chunked data, relative context)

---

## 3. Be honest and predictable

Interfaces earn trust through consistency. The same action should produce the same result. Buttons should look like buttons. Destructive actions should feel weighty. Feedback should be immediate and truthful. If something is AI-generated, or if a workflow has data-handling limits, say so clearly.

*In ScribeFloat*: Trust is load-bearing. Users are capturing sensitive conversations and relying on the accuracy of transcripts.

→ Connects to: `interaction.md` (AI labelling, data clearance), `data-display.md` (AI output, states)

---

## 4. Design for the real context

Know who your users are, what device they are on, how much time they have, and what they are trying to accomplish. In ScribeFloat's context: a knowledge worker on a laptop, under time pressure, capturing conversations quickly and reliably.

*In ScribeFloat*: Laptop-first. Dense but scannable. Fast and frictionless over spacious and decorative.

→ Connects to: `user-context.md`, `visual-design.md` (responsive, breakpoints)

---

## 5. Progressive disclosure over upfront complexity

Show what is needed at each step. Let complexity reveal itself as the user goes deeper. Break long processes into stages. Use sensible defaults so most users never need to touch advanced settings.

*In ScribeFloat*: Onboarding, settings, and multi-step flows. Never front-load decisions the user doesn't need yet.

→ Connects to: `interaction.md` (filters, explanations), `data-display.md` (states)

---

## 6. Feedback is non-negotiable

Every interaction needs visible feedback. Clicked a button? Show it. Submitted a form? Confirm it. Something failed? Explain it where it failed, not in a toast 300px away.

*In ScribeFloat*: Users need to know the recording started, the file was saved, the transcript is processing. Silent failures are unacceptable.

→ Connects to: `interaction.md` (buttons, feedback placement), `data-display.md` (error/loading/success states)

---

## 7. Optimise for the common case

Lead users clearly through the main workflows, then keep secondary actions, settings, and edge-case options available with less visual weight. Do not make everything compete equally for attention.

*In ScribeFloat*: The main path (record → transcript → review) should be dominant at every step. Settings and advanced controls exist but recede.

→ Connects to: `visual-design.md` (hierarchy, surfaces), `typography.md` (scale, emphasis)

---

## 8. Use familiar patterns

Do not invent novelty for its own sake. If users already know how to work through something in tools they use every day — Notion, Slack, native macOS apps — borrow those patterns where they fit so the product is faster to learn.

*In ScribeFloat*: Jakob's Law. Familiar patterns reduce the cost of switching to and from ScribeFloat in a multi-tool workflow.

→ Connects to: `ux-laws.md` (Jakob's Law), `interaction.md`

---

## 9. Show the explanation when it matters

In high-stakes workflows, time is too valuable to make the interface cryptic. If someone needs context to use a feature properly, show it in the interface. Do not hide essential explanation in tooltips.

*In ScribeFloat*: AI output, data clearance notices, and workflow-specific constraints must be visible where the user is making the decision.

→ Connects to: `interaction.md` (inline explanations, AI data), `data-display.md` (AI labelling)

---

## 10. Absorb complexity on the engineering side

Every system has complexity. The question is whether the user carries it or the system does. Accept flexible inputs, handle edge cases well, and do the hard work so the interface stays simple.

*In ScribeFloat*: Audio format handling, model selection, transcript correction — these are engineering problems, not user problems.

→ Connects to: `ux-laws.md` (Tesler's Law), `interaction.md` (input handling)

---

## How the principles connect

Principles 1, 2, and 7 govern **economy** — every element must justify its presence.
Principles 3, 6, and 9 govern **trust** — the product must be honest and responsive.
Principles 4, 5, and 8 govern **context** — design for the real user doing real work.
Principle 10 governs **engineering responsibility** — carry the weight yourself.

→ **Next**: Pick the domain file that matches your task — see `SKILL.md` quick-navigate table
→ **Laws behind the principles**: `ux-laws.md`
