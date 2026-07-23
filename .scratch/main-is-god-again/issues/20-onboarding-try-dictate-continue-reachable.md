---
title: Onboarding Try Dictate Continue reachable
labels: [wayfinder:task, needs-triage]
status: open
assignee:
blocked_by: []
parent: MAP.md
---

## Question

On the onboarding “Try Dictate” step, can a user always reach Continue after a successful practice capture — including long pasted/dictated text — without an opaque “send N more messages” gate trapping them?

**Done when:** After at least one successful practice Dictate (or an explicit Skip), Continue stays visible and usable; long transcript text cannot push footer actions off-canvas; any practice gate is obvious in the UI (or removed).

## Why merge-blocker

First-run includes finishing Setup. Human hit a long Dictate during practice: history overflow buried / removed Continue; progress only returned after several more short sends (suspected ~4-message gate). That is a real trap on the ship-bar first-run path, not polish.

## Seen

Silicon ship-bar smoke onboarding Try Dictate (2026-07-21). Short capture worked; large text filled the preview and Continue disappeared until more messages were sent.

## Likely fix direction (not to-spec yet)

- Clip / scroll the practice preview so footer Back / Continue stay in layout
- Gate Continue on one successful capture (or allow Skip), and surface the rule in copy if a multi-try gate stays
- Do not require four sends as a silent condition

## Out of scope here

- Gamifying double-tap vs tap-and-hold (Known issues)
- Weird practice timestamp display (Known issues)
- Cold Whisper preload on first practice (Known issues)
---
