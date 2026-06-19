---
type: context
topic: design-intent
when: Starting a new screen; unsure whether a decision fits the product; reviewing whether something "feels like ScribeFloat".
see-also:
  - principles.md — the WHY behind these design choices
  - checklist.md — the litmus test in checklist form
---

# User Context & Design Intent

> **When to read this**: You're starting a new screen, component, or flow and need to ground yourself in who this product is for and how it should feel.

ScribeFloat is a professional tool for knowledge workers who capture conversations, record transcripts, and dictate text. The product should feel like a precision instrument — not a consumer app.

---

## Know your user

Before building anything, answer these four questions:

**Who is this for?** Knowledge workers on laptops, capturing conversations quickly so they can focus on the conversation rather than note-taking. Primarily power users who will run this tool dozens of times a week.

**What's the task?** Recording, transcribing, reviewing transcripts, dictating, or managing captured content. Fast, repeatable, focused work.

**What density?** Comfortable density for power users. These users work with a lot of text and data — wasted screen space means wasted productivity.

**What does success look like?** The user accomplishes their task faster and with more confidence than before. Speed and reliability are the primary signals of success.

---

## How ScribeFloat tools should feel

**Professional and trustworthy** — a serious tool for serious work, not a SaaS dashboard or a consumer app.

**Neutral and calm** — restrained, clear, and dependable. Not loud, over-designed, or trying to delight through novelty.

**Snappy and frictionless** — common workflows move immediately, without clunky steps or unnecessary waiting.

**Quietly premium** — polished and refined, never flashy. The product should feel finished, not decorated.

**Dense but scannable** — information-rich interfaces where users can find what they need without scrolling through sparse cards. Every pixel earns its place by conveying useful information.

**Warm but serious** — the accent brand colour softens the neutral base without making the product feel generic.

**Consistent** — the same things work the same way across the product. Consistency is how trust is built over repeated use.

---

## What ScribeFloat tools must avoid

| Anti-pattern | Why it fails here |
|---|---|
| Consumer-app aesthetics — playful illustrations, bubbly shapes, gamification | Undermines trust. Knowledge workers need to feel they are using a professional instrument. |
| Marketing-site patterns — hero sections, gradient backgrounds, testimonial cards | These are working tools, not products being sold. |
| Sparse layouts with excessive whitespace | Wastes screen real estate that power users need. Knowledge workers do not want to scroll through cards with 32px padding to find what they're looking for. |
| Overly creative typography or decoration | Brand fonts are fixed. Decoration competes with content. |
| Multiple accent colours or rainbow status indicators | Reserve **brand** for the single primary CTA per view. Each status colour has a defined semantic job — no decorative misuse. |

---

## The litmus test

> At 11pm, would a product designer open this without hesitation, confident it will be fast, clear, and worth using? Does it feel neutral, trustworthy, and snappy, with the important information visible and the workflow frictionless? Does it feel like a tool built for knowledge workers rather than a generic SaaS product?

If the answer is "not quite", look at: hierarchy (are the most important things visually dominant?), density (is there wasted space?), speed (does every action feel immediate?), and trust (is the product being honest about what it's doing?).

---

## ScribeFloat design DNA

These are the product-specific interpretations of common UX advice. When generic guidance conflicts with these, the DNA wins.

| Generic advice | ScribeFloat interpretation |
|---|---|
| "Use whitespace generously" | Use whitespace *intentionally*. Wasted space means wasted productivity for knowledge workers. |
| "Keep it simple" | Simple does not mean sparse. A well-organised dense interface is easier to use than a sparse one that forces extra paging. |
| "Delight the user" | Respect the user. Delight here means speed, accuracy, and reliability — not confetti or animations. |
| "Mobile-first" | Laptop-first. Primary users are on laptops or large monitors. The UI should hold up at narrower widths, but mobile is not the main case. |
| "Minimalist aesthetic" | Information-rich aesthetic. Every pixel should earn its place by conveying useful information. |

---

→ **Next**: If you're asking *why* these feel-rules exist, read `principles.md`
→ **Before shipping**: run the `checklist.md` litmus test checks
