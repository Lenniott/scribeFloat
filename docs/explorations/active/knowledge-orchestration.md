# Knowledge Orchestration — Design Exploration

> Status: **active exploration — significant pivot recorded below**. Decisions below are agreed in conversation and captured here to avoid living only in chat history. Open questions are explicitly marked.
> Prerequisite reading: [`knowledge-layer-intent.md`](knowledge-layer-intent.md), [`design-brain-prd.md`](design-brain-prd.md)

---

## The problem

> A designer with ADHD does their best thinking out loud — in recordings, in conversations, while dictating. That thinking is real and good. But it evaporates. Decisions are not documented. Artefacts are hard to create because everything is bundled in memory after conversations. Priorities get missed. No preparation means no understanding of what stakeholders actually need — so the designer does it their way, not the way that makes sense.

| Problem | ScribeFloat's answer |
|---|---|
| Thinking evaporates after conversations | Ambient capture — voice, no new habit required |
| Decisions not documented | Float tags and annotates at capture time |
| Can't prepare for stakeholders | Extract context file for relevant tags, take to AI |
| Artefacts bundled in head, hard to produce | Extract context file, take to AI to produce the artefact |
| Priorities missed | Starred tag logs surface what matters across all notes |
| Artefacts speak designer's language not stakeholder's | Float captures exact stakeholder words via grep; context files carry that language to the AI producing the artefact |

**Design principle:** Float should prefer quoting over summarising. The annotation interprets (why this tag, why this matters). The grep preserves (exactly what was said). Interpretation without the quote is lossy. The quote without interpretation is noise.

**Success metric:** time spent dictating and transcribing vs. typing. Any feature that requires typing, filing, or explicit organisation is a regression.

---

## What's been decided

### 1. Storage format

Markdown. One file per knowledge item — not one file per type.

**Why one file per item:**
- Progressive disclosure: an agent reads a folder index (cheap) then loads only the items it needs
- Easy to deprecate: change status in frontmatter, or move to `deprecated/`
- Easy to update: modify one file without touching others
- Temporal validity: each item carries its own validity metadata
- Diffable: git diff shows exactly what changed about one decision

### 2. Folder structure

```
{save-folder}/
  knowledge/
    AGENTS.md                              ← root index: all domains + user
    {domain-name}/
      AGENTS.md                            ← domain index: all 12 type folders
      decisions/
        AGENTS.md                          ← type index: all items + status
        {slug}.md                          ← one file per item
      actions/
        AGENTS.md
        {slug}.md
      stakeholders/
        AGENTS.md
        {slug}.md
      workflows/
      glossary/
      facts/
      questions/
      mental-models/
      personas/
      ideas/
      results/
      goals/
    user/
      AGENTS.md
      strengths/
        AGENTS.md
        {slug}.md
      weaknesses/
      voice-and-tone/
      philosophy/
      communication/
      network/
      interests/
      blind-spots/
      biography/
```

The `knowledge/` folder lives inside the user's ScribeFloat save folder alongside notes, transcripts, and audio.

**The 12 domain information types:**
Workflows, Stakeholders, Glossary, Facts, Questions, Actions, Decisions, Mental Models, Personas, Ideas, Results, Goals

**User model categories:**
Voice and tone, Philosophy, Strengths, Weaknesses, Communication, Network, Interests, Blind spots, Biography

**User model rule:** User knowledge is populated **only from dictation**. Dictation is the only reliable source of the user speaking about themselves. Written notes and uploads do not feed the user model.

### 3. Navigation — AGENTS.md hierarchy

Three levels of AGENTS.md, consistent convention at every level. Any agent cold-starting in the knowledge folder reads one file and knows where everything is.

**Root `knowledge/AGENTS.md`** — lists all domains with one-line descriptions and status.

**Domain `{domain}/AGENTS.md`** — lists all 12 type subfolders that exist for this domain.

**Type `{domain}/{type}/AGENTS.md`** — lists all items in this folder with status and description.

**Entry format** (consistent at every level):

```markdown
- [filename.md](filename.md) `active` — one-line description of this item
```

Status values in the entry tag: `active` / `deprecated` / `complete` / `backlog`

### 4. Index maintenance — scripts, not agents

AGENTS.md files are maintained by deterministic scripts, not by an LLM. The LLM provides a minimal JSON schema; the script handles all file and index operations.

**Minimal schema the agent outputs:**

```json
{
  "filename": "go-with-option-a",
  "folder": "knowledge/project-x/decisions/",
  "description": "Chose Option A over B for navigation redesign — accessibility constraints"
}
```

**What the script does from there:**
1. Creates `{folder}/{filename}.md` with frontmatter stub
2. Appends entry to `{folder}/AGENTS.md`
3. Creates folder and appends to `{domain}/AGENTS.md` if the type folder is new
4. Creates domain folder and appends to `knowledge/AGENTS.md` if the domain is new

Removal: script removes the entry from AGENTS.md at all levels and optionally moves the file to a `deprecated/` subfolder.

Scripts are triggered on file creation and deletion. Index never drifts.

### 5. Source linking

Every knowledge item links back to the exact location in the source note where it was extracted from — not just the file, but the moment within a long transcript.

**Frontmatter source reference:**

```yaml
sources:
  - path: notes/transcripts/2026-06-20-session.md
    timestamp: "00:14:32"
    grep: "option A accessibility"
```

- **`path`** — the source note file path
- **`timestamp`** — Whisper segment timestamp; primary jump point into a long transcript
- **`grep`** — 2–4 distinctive words from within the passage; locates the exact sentence without asking the agent to reproduce verbatim text (agents misquote; short grep patterns locate reliably)

For uploads without Whisper timestamps (PDF, URL, video): `grep` only, no `timestamp`. Longer pattern acceptable.

**Why not verbatim excerpt:** asking an agent to reproduce exact transcript text is asking it to do what it does worst — precise recall. Short grep pattern is a much safer ask that achieves the same navigation goal.

### 6. Agent architecture — routing, not per-domain proliferation

**Wrong model:** one agent per domain × type = combinatorial explosion (5 domains × 12 types = 60 agents before the user adds anything custom).

**Right model:**
- **1 routing agent** — classifies a note to one or more domains using Float's tag/keyword output; builds a confidence score; determines which domain folders exist or need creating
- **12 type agents** — one per information type, domain-agnostic; each knows how to extract its type from any note; domain is a routing parameter
- **Scripts** — handle all file/index maintenance

The type agents are reusable across every domain. The routing agent uses vocabulary Float already produces — no separate classification LLM call needed in most cases.

---

## The full flow — scenario walkthrough

**Note composition:** speaker capture (mic + speaker), written notes taken during the call, uploaded PDF (external brief or spec). Project X, stakeholder conversation.

### Phase 1 — Note assembly (deterministic, existing Rust)

```
Mic audio + Speaker audio → Whisper → timestamped transcript
Written notes → already text
PDF upload → markdown conversion
```

Three Sources on one Note. Critically, Sources stay **labeled by type** — they are never merged into flat text. Agents receive:

```markdown
## Transcript
[timestamped speaker content...]

## Written Notes
[user's typed notes...]

## Upload: design-brief.pdf
[PDF markdown content...]
```

### Phase 2a — Tags + Keywords (lightweight, runs on every note)

The `on-creation` Flow triggers the Tags + Keywords step. These are a **linking and filtering layer** — lightweight vocabulary that helps organise notes, link them together, and provides the domain routing signal for Phase 3. This is not where Decisions or other knowledge types live.

Status = `draft`. **User approves.** Nothing in Phase 2b runs until approved — approved tags and keywords are the routing signal. Running knowledge extraction against unvalidated output compounds errors.

### Phase 2b — Domain routing (uses approved tags as signal)

See Phase 3 below. Tags/Keywords feed this step directly.

### Phase 3 — Routing + Knowledge extraction (separate flow, selective)

The routing agent reads approved tags + keywords, cross-references against `knowledge/AGENTS.md`, scores domain confidence:

```
"project-x"    → 94%  (tag in vocabulary, stakeholder name matched)
"ux-practice"  → 61%  (tags "user research", "stakeholder")
```

- High confidence (>80%) → proceed automatically
- Low/ambiguous → queue for user confirmation: *"Is this Project X?"*

Domain routing is mostly string matching against existing vocabulary — not a separate LLM call. LLM only enters when confidence is ambiguous or the domain doesn't exist yet.

### Phase 4 — Type extraction (12 agents, independent per domain)

For each routed domain, each type agent runs independently. Each receives: the labeled note content, the domain it's operating in, and the current `{domain}/{type}/AGENTS.md` (to avoid duplicating existing items).

**Critical distinction — two classes of type agent:**

| Class | Types | Sources they read |
|---|---|---|
| **Session-only** | Decisions, Actions, Questions, Ideas, Results | Transcript + Written Notes only |
| **Context types** | Facts, Glossary, Stakeholders, Workflows, Personas, Mental Models, Goals | All sources including uploads |

Uploads are **context brought into** a session, not decisions made in it. A decision found in a PDF is something decided before — potentially the thing the meeting is overturning. Flattening sources would produce conflicting knowledge items with no signal about which supersedes which.

**Upload decisions get `source_type: referenced`, not `source_type: decided`.** The brief's stated direction is worth capturing — it's the baseline the meeting either confirmed or overturned — but it must not be conflated with decisions made live in the session.

**Example output for this scenario:**

| Agent | Finds | Output |
|---|---|---|
| Stakeholders | "Sarah, lead designer at Acme" | `sarah-acme.md` |
| Decisions | "agreed on card-based layout" (transcript) | `card-layout-direction.md` — `decided` |
| Decisions | "go with list view" (PDF brief) | `list-view-direction.md` — `referenced` |
| Actions | "Ben to send prototype by Friday" | `send-prototype-friday.md` — status: upcoming |
| Questions | "unclear on approval process" | `approval-process-unclear.md` |
| Facts | "client has 3 internal designers, uses Figma" | items |
| Ideas | "could try card sorting next session" | `card-sorting-idea.md` |
| Workflows | nothing found | — |

Each agent outputs the minimal JSON schema; scripts handle file creation and AGENTS.md maintenance.

### What's deterministic vs. LLM

| Step | Who |
|---|---|
| Note assembly | Rust (existing) |
| Float Phase B extraction | LLM via existing engine |
| Domain routing — high confidence | String matching against vocabulary |
| Domain routing — ambiguous | LLM confidence scoring |
| Type extraction content | LLM (each type agent) |
| JSON schema output | LLM (constrained format) |
| File creation + AGENTS.md update | Script (deterministic) |
| Source timestamp | Whisper (deterministic) |
| Source grep pattern | LLM picks 2–4 words from the passage |

---

## The engine pattern

Two flows, both built on the same underlying runner:

**Flow 1 — Tags + Keywords** (per-note, lightweight, always on)
```
Layer definition + prompt
→ Step runner (HTTP call to inference endpoint)
→ Vocabulary items on HistoryRecord (draft → approved)
→ Feeds domain routing signal
```

**Flow 2 — Knowledge extraction** (per-note × domain, heavier, selective)
```
Type definition + prompt
→ Type agent runner (HTTP call to inference endpoint)
→ Structured JSON schema (filename, folder, description, source link)
→ Script → markdown file + AGENTS.md update
```

Tags and Keywords are **not** knowledge types — they are the index that makes knowledge extraction possible. Decisions, Actions, Stakeholders, etc. live in Flow 2, not Flow 1. This was blurred in the design-brain-prd.md which listed Decisions alongside Tags/Keywords as Layer types — that framing is superseded here.

The runner is the same HTTP inference client in both cases. Storage differs: HistoryRecord metadata for Flow 1, markdown files for Flow 2.

ScribeFloat ships with defaults at both levels:
- Flow 1 defaults: Tags, Keywords
- Flow 2 defaults: the 12 type agents (Decisions, Actions, Stakeholders, etc.)

A user adding a custom knowledge type — "Risks", "Open Source References" — defines a prompt describing what to extract. No new code, just configuration.

**Scalability:** adding a domain costs zero (scripts handle folder structure). Adding a type costs one prompt definition. Running 10 domains × 12 types = 120 independent, bounded, parallel LLM calls each producing the same JSON schema shape.

---

## Open questions

These are the next branches of the design tree to resolve, in dependency order:

1. **Individual item frontmatter schema** — what fields does every knowledge item file carry? (type, domain, status, created, updated, sources are likely; what else?)

2. **Time dimension implementation** — Now / Upcoming / Scheduled / Backlog / Complete. Is this a `status:` field on Actions and Goals items, or does it live somewhere else? Does it cut across all 12 types or only some?

3. **New domain creation flow** — who or what creates a new domain? Does the routing agent create it on first encounter, or does the user explicitly name domains? What happens when confidence is low — does the routing agent ask?

4. **Temporal validity** — how does a knowledge item go stale? (A decision made in March may be superseded in September.) Is there a `valid_until:` or `superseded_by:` frontmatter field? Who sets it?

5. **Proactive surfacing** — the system should inject relevant context when a new session starts ("last time on this project: X was decided"). Where does this live architecturally — inside Float's per-session processing context, or as a separate pre-session step?

6. **Voice query** — "what did we decide about onboarding" as a voice dictation that routes to a bounded knowledge query and returns a grounded short answer. This is not a search interface. How does it trigger and what does it return?

---

## Pivot — simplified architecture

After designing the full knowledge layer (domain folders, 12 type agents, AGENTS.md hierarchy, routing orchestration), a simpler approach emerged that supersedes most of it.

### Core principle

**ScribeFloat is the capture and context layer. The user brings the capable model.**

Users have access to capable AI elsewhere (Claude, ChatGPT, etc.) for synthesis, analysis, and document generation. ScribeFloat's job is not to synthesise knowledge — it is to produce well-structured, information-dense context that a capable model can work with. The division:

```
ScribeFloat                          →   User's AI of choice
capture + tag + annotate + export    →   synthesise, analyse, write, decide
```

### What this replaces

The knowledge layer (domain folders, 12 type agents, AGENTS.md maintenance, routing orchestration, proactive surfacing) is replaced by a much simpler pipeline:

1. **Capture** — transcript, written notes, uploads (existing)
2. **Tag + annotate** — Float Phase B adds tags and keywords, and for each tag writes *why* it was applied and what in the note relates to it (small addition to existing Step output)
3. **Export context file on demand** — user requests a context file for one or more tags over a time window; system collates annotations + source quotes into one markdown file per tag; user takes it to their AI tool

### Tag annotation

When Float tags a note, it also writes a short annotation per tag:

```
tag: project-x
annotation: Stakeholder meeting for Project X. Sarah approved card layout over list view.
source: timestamp 00:23:14, grep "card-based agreed layout"
```

This annotation is stored alongside the tag on the HistoryRecord — not in a separate knowledge folder.

### Context file extraction (on demand)

User specifies: tags + time window. System:
1. Finds all notes tagged with those tags in the window
2. Pulls the annotation Float wrote for each note
3. Pulls the quoted passage using the grep/timestamp reference
4. Assembles one markdown file per tag:

```markdown
# project-x — context (last 3 months)

## 2026-06-20 · stakeholder-call-sarah.md
> "card-based agreed layout"
Float: Stakeholder meeting for Project X. Sarah approved card layout over list view.

## 2026-05-14 · design-review.md
> "project x timeline unclear"
Float: Team discussion flagged timeline uncertainty for Project X phase 2.
```

Deterministic assembly — no LLM call at extraction time. Fast, no hallucination risk. The LLM work already happened at tag time.

### Context file storage

Saved to `knowledge/exports/` with a datestamp. Not maintained artifacts — outputs the user grabs and takes elsewhere. The system doesn't need to know they exist after writing them.

### What survives from the domain/type design

The domain folder structure and AGENTS.md navigation is **deferred** — not abandoned. If the user ever wants to maintain structured artifacts long-term (a stakeholder directory, a running decision log), the folder structure is the right place for those. But that's a user-driven action, not something Float maintains automatically.

The annotation format (timestamp + grep) and the source-type distinction (session-only vs context types, `decided` vs `referenced`) still apply to the tag annotation system.

---

## Further simplifications

### Tags and keywords merge into one concept

Tags and keywords are the same thing — one list, one concept. Each tag:

```
tag
  name
  description        — what this tag means globally
  logs[]
    date
    note_id          — which note
    timestamp        — Whisper segment (primary jump point)
    grep             — 2–4 words to locate the passage (not verbatim)
    status           — starred | recent | archived
```

Status on logs: `starred` = user flagged as high signal; `archived` = deprioritised but kept; `recent` = automatic. When generating a context file, starred entries surface first.

### CLI for terminal agent access

If ScribeFloat exposes a CLI, terminal agents (Claude Code, etc.) can interact with the data directly — no proprietary extraction layer needed inside the app:

```bash
scribefloat tags list
scribefloat context --tags project-x,sarah-acme --since 90d
scribefloat notes read <id>
```

The agent provides intelligence; the CLI provides the interface. A terminal agent can read tag logs, pull passages, and produce whatever artifact it needs without ScribeFloat building its own extraction layer.

**What Float's annotation still provides:** it runs at capture time, when context is fresh. A terminal agent reading a raw hour-long transcript months later has to re-derive what Float already noted. The grep + timestamp anchor is Float's durable contribution — the CLI just surfaces it.

Architecture: Float annotates at capture time → CLI exposes the data → terminal agent does the work.

---

## Loose thoughts

- **No collective label for child types.** When a user opens a Domain they see the types directly (Decisions, Stakeholders, etc.) — no intermediate "Knowledge" or "Entries" label needed. The type names are the navigation. "Project X → Decisions → card-layout-direction." The types speak for themselves.

- **User-facing language.** The technical parent/child/agent model maps to: Domain (where it lives) → type name (what it is) → invisible agent. "User" as a parent becomes "You" or "Your profile" — personal not technical. Agents are never surfaced to the user.

- **Time is a property, not a container.** Temporal status (Now, Upcoming, Scheduled, Backlog, Complete) is inferred from content by the agent — not filed by the user, not a folder in the structure. "Send prototype by Friday" → `status: upcoming`. No `knowledge/time/` folder.

---

## Reference

| Document | Relationship |
|---|---|
| [`design-brain-prd.md`](design-brain-prd.md) | Float enrichment engine — prerequisite; produces the tags/vocabulary the routing agent uses |
| [`knowledge-layer-intent.md`](knowledge-layer-intent.md) | Earlier intent doc — this exploration supersedes its open questions |
| [`docs/architecture.md`](../architecture.md) | System context — knowledge folder lives in the save folder alongside history.jsonl |
