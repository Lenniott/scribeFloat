# Accumulated Context — Problem Statement, User Profile, and Use Cases

> Status: **exploration / no solutions**. This document captures the problem space and who it belongs to. No storage model, data structure, or feature is proposed here.

---

## Problem statement

A designer who records their thinking — meetings, stakeholder conversations, user research sessions, personal dictation — accumulates a corpus of context over time. The corpus grows, but it stays inert. When a task arrives that could be informed by that accumulated context, the designer has no way to access it usefully. They know the relevant conversations happened. They cannot easily turn them into the thing they need next.

The bottleneck is not capture. It is **retrieval and assembly**: getting the right slice of accumulated context into the right shape for the task at hand.

This matters most when the task involves an external collaborator — another person, or an AI model. The designer needs to hand off context, not just hold it. Today that handoff is manual: they re-read notes, copy fragments, reconstruct the picture from memory. The reconstruction step is where value leaks.

A secondary problem: the designer's accumulated context spans multiple concurrent threads — projects, client relationships, research programmes, personal concerns — and these threads interact. A decision in one thread has implications in another. Nothing currently makes those connections visible.

---

## User profile

**Primary: the reflective solo designer**

A UX or product designer who works across multiple concurrent projects and relationships. They think out loud — in calls, in dictation, in informal conversations — and they capture that thinking through ScribeFloat. They are not primarily a note-taker; they are a practitioner whose work produces artifacts: research surveys, job roles, design briefs, stakeholder decks, specs.

Characteristics:
- Works with stakeholders (directors, clients, research participants) whose intent and language matters and needs to be preserved, not paraphrased
- Runs structured research processes (surveys, interviews, synthesis) where the output must be traceable back to source material
- Uses AI models as working partners for drafting and refining artifacts — not just as search tools
- Carries multiple active concerns simultaneously: project A, client B, a hiring process, a personal design direction
- Does not manage a knowledge base — has no habit of filing, tagging, or organising after the fact; any system that requires upfront organisation will fail

What they are not:
- A researcher who treats their notes as a database
- A manager running a team with shared documentation
- Someone who enjoys system maintenance

---

## Use cases

These are situations the designer actually encounters, described at the level of intent and frustration — not at the level of features.

---

### UC-1: Crafting a research instrument from stakeholder intent

The designer is preparing a Maze usability survey. Before writing questions, they spoke with two stakeholders to understand the research intent — what decisions the survey should inform, what assumptions need testing, what the team already believes. Those conversations happened in ScribeFloat sessions over the past two weeks.

When they sit down with an AI to draft the survey, they want the AI to have that stakeholder context. Right now they have to re-read the transcripts, extract the relevant intent manually, and paste it in. The manual step breaks flow and risks leaving out something that mattered.

**The gap:** there is no way to say "here is what I know about the intent behind this survey" without reconstructing it by hand.

---

### UC-2: Writing a job role from a director conversation

The designer has a transcript of a conversation with a director about a hiring need — the gap in the team, the kind of thinking they need, the business context. They also have a rough draft of the job description. They want to use an AI to refine the draft using the director's actual words and intent.

The director's exact language matters. "We need someone who can hold ambiguity while the strategy is forming" is more useful than a paraphrase. The designer wants to ground the artifact in the transcript, not in their memory of it.

**The gap:** the transcript exists but there is no way to make it available as context for a focused task without manually lifting quotes and pasting them in.

---

### UC-3: Backing design decisions with user evidence

The designer has run eight user research sessions over three months. They are writing a design rationale and want to include user quotes that support specific decisions. They know the relevant things were said — they were in the room — but finding the exact quote across eight transcripts is a manual search problem.

The artifact needs to be evidence-based, not just asserted. "Users found the onboarding overwhelming" is weaker than the actual words a user used when they encountered it.

**The gap:** there is no way to say "find me moments where users expressed confusion about onboarding" across a body of transcripts.

---

### UC-4: Understanding the current state of a thread

The designer has been working on a client project for four months. Multiple calls, multiple decisions, multiple open questions. Before a meeting with the client they want to know: where are we, what has been decided, what is unresolved, what does the client care about.

They do not want to re-read four months of transcripts. They want a current picture, not an archive.

**The gap:** the accumulated record tells them what happened but not where they are. The record is not the same as the current state.

---

### UC-5: Knowing what to do next

After a busy week of calls the designer has new information, new commitments, and new open questions. Some of these are urgent; some belong to specific projects; some belong to their personal thinking. They want to know: given everything that has been said this week, what needs to happen?

This is not a search problem. It is a synthesis problem. The designer cannot hold all threads in mind simultaneously and know which needs attention.

**The gap:** the capture is complete; the synthesis from capture to action does not happen.

---

### UC-6: Sharing context with a collaborator or external model

The designer is handing work to another person — a contractor, a collaborator, an AI — who has none of the background. They need to brief that person efficiently without a two-hour call. The relevant context exists in the corpus but is not in a form that can be handed over.

The briefing should draw on actual conversations, not on the designer's reconstruction of them. It should cover: what this is, why it matters, what has been decided, what is still open.

**The gap:** there is no way to produce a briefing from the corpus without writing it from scratch.

---

## What these use cases have in common

Every use case has the same shape:

1. Context was captured over time in conversations
2. A task arrives that could be informed by that context
3. The relevant context is a subset of the corpus, not all of it
4. The context needs to be in a usable shape — not raw transcripts
5. The output goes somewhere: into an artifact, into a handoff, into a model session

The value is not in having captured. It is in being able to use what was captured, at the moment it is needed, without reconstructing it manually.
