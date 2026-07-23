---
type: patterns
topic: data-display
when: Building any view with data, AI output, or state-dependent UI (empty, loading, error, success).
see-also:
  - principles.md — #3 (honest), #6 (feedback), #9 (show the explanation)
  - interaction.md — feedback placement for actions
  - visual-design.md — skeleton screen patterns, surface tokens
---

# Data Display

> **When to read this**: You're building any data-driven view — lists, transcripts, AI output, search results — or you need to handle empty, loading, error, or success states.

Data presentation is not about putting numbers on a screen. It is about making meaning, relevance, and next steps obvious. A number alone is not enough — the question is what it means and what the user should do next.

→ **Principles at work**: #3 (honest), #6 (feedback), #9 (show the explanation when it matters)
→ **Feedback placement mechanics**: `interaction.md`
→ **Skeleton screen implementation**: `visual-design.md`

---

## Formatting

Use `tabular-nums` so digits align in columns. This is critical for timestamps, durations, and file sizes.

Format long IDs and numbers in readable chunks (matter IDs, reference numbers, large monetary amounts).

Show relative context — percentages, trends, comparisons — not just raw values. *"42 of 156 documents reviewed, 27%"* is more useful than *"42 documents"*.

Use `truncate` or `line-clamp` for long text in dense layouts — document titles and names can be extremely long.

---

## AI-generated information

If something is AI-generated, label it clearly. Do not make users infer that a summary, draft, answer, score, or recommendation came from AI.

Show what the AI result is based on where possible: source documents, cited inputs, timestamps, confidence signals.

Make it clear when the user still needs to verify the result for accuracy.

Do not present AI output with the same visual certainty as confirmed facts, system records, or human-reviewed content unless it has actually been checked.

---

## Data clearance

State clearly what kinds of information the app is approved to handle.

If users are allowed to enter confidential or private information, say so clearly near the point of entry. If they are not, say that equally clearly.

Put data clearance guidance near inputs, uploads, and workflows where the decision matters — not in a help article.

---

## Empty, loading, and error states

Every data-driven view needs all three. Knowledge workers will hit all three often.

| State | Guidance | ScribeFloat example |
|---|---|---|
| **Loading** | Structural skeletons matching the layout they'll replace | Document list skeleton with rows, not a spinner |
| **Empty** | Clear message and one obvious next action | "No recordings yet. Start a new recording to capture a conversation." |
| **Error** | Explain what happened, show it at the point of failure, offer retry | "Failed to load transcript. Retry" inline, not a page-level error banner |

**Skeleton screens** must match the layout of the populated state — same number of rows, same proportions. Generic spinners tell the user nothing about what's coming.

**Empty states** must do two things: explain why it's empty, and offer the obvious next step. A blank space with no guidance is a dead end.

**Error states** must appear at the point of failure. A form error should highlight the failing input. A data load failure should appear in the data area, not in a separate toast.

---

## Feedback placement

**Success**: show relative to the trigger (checkmark on the button, not a distant toast). A "copy to clipboard" action gets a temporary checkmark on the copy button.

**Errors**: highlight the specific input or data area that failed. Do not flag the entire form when one field is wrong.

**Progress**: show progress bars, step indicators, or completion percentages for multi-step processes. Motivation increases as users get closer to their goal (Goal-Gradient Effect — see `ux-laws.md`).

---

→ **See also**: `interaction.md` — buttons showing loading state, optimistic updates, and stale request cancellation
