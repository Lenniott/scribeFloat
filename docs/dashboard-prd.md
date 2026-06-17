# Dashboard & Navigation Shell — PRD

> Companion to [`design-brain-prd.md`](design-brain-prd.md).  
> Mockup reference: `docs/mockup-dashboard.html` on `spike/design-brain-v1`.

---

## 1. What this is

A new application shell for ScribeFloat — replacing the existing History window with a proper home screen, a restructured sidebar, and a more capable Transcripts view. This work is fully independent of Float (the AI processing engine). It can be scoped, built, and shipped before a single line of LLM code is written.

---

## 2. The bet

The current History screen is a flat chronological list. It has no overview, no way to understand recording activity at a glance, no filter by content, and no quick path to start a new session. For a user who records regularly, it becomes a scroll problem rather than a knowledge surface.

The new shell bets that a home screen and a smarter Transcripts view make the app feel complete even without AI — and that they set the right stage for Float to slot in later without any retrofitting.

---

## 3. Phasing

### Phase A — no Float required (this PRD's scope)

| Screen | What's included |
|---|---|
| Dashboard | Greeting, stats tiles (without Float-specific counts), recent sessions |
| Sidebar | Capture section, Manage section, Dictate in title bar |
| Transcripts | Source-type filter tabs, tag filter side panel using existing `HistoryRecord` tags |
| Float nav item | Present but dormant (greyed, labelled "coming soon", not navigable) |

### Phase B — requires Float

Covered in [`design-brain-prd.md`](design-brain-prd.md). Phase B activates the Float nav item and adds enrichment status, Draft/Approved workflow, per-layer vocabulary, and the filter panel's additional layer groups.

---

## 4. Sidebar

Two sections — **Capture** and **Manage** — plus a footer.

### Capture

| Item | Behaviour |
|---|---|
| **Scribe** | Launches the existing Scribe floating recorder window. Amber accent — the primary action. Does not navigate in the main window. |
| **Upload** | Opens a file-import flow in the main window (import audio → transcribe). |

### Manage

| Item | Phase A | Phase B |
|---|---|---|
| **Dashboard** | Active | Active |
| **Transcripts** | Active | Active |
| **Float** | Greyed, "coming soon" badge | Active |

### Footer

- Settings gear
- Status line: `local · no cloud / Gemma E2B · queue 1` (queue line hidden in Phase A if Float isn't loaded)

### Dictate

Dictate is **not** a sidebar nav item. It is a persistent trigger button in the title bar — always visible regardless of which screen is shown. Clicking it opens the Dictate HUD. It never navigates in the main window. This matches the existing architecture where Dictate operates as a separate overlay HUD.

Rationale: Dictate is an action, not a destination. Putting it in the nav implies it has a screen to go to — it doesn't.

---

## 5. Dashboard screen

### Header
Greeting (`Good afternoon, Ben`) + current date. Uses the system clock.

### Stats row

Four tiles in a grid:

| Tile | Phase A value | Phase B value |
|---|---|---|
| Transcripts | Total count from history | Same |
| Float layers | Hidden or `—` | Count of configured layers |
| Drafts to review | Hidden or `—` | Count of records with any layer in Draft status |
| Recorded this week | Sum of session durations from current ISO week | Same |

"Recorded this week" requires a query over `HistoryRecord.duration_secs` filtered by date. If not implemented in Phase A, show `—`.

### Recent list

Last N sessions (suggested N = 5–8, configurable later). Each card shows:
- Source type icon (Scribe / Dictate / Upload)
- Title and excerpt
- Tag chips from `HistoryRecord.tags` (existing field)
- Float enrichment status chip — hidden in Phase A, shown in Phase B
- Timestamp and duration

"See all →" link navigates to Transcripts.

---

## 6. Transcripts screen

### Layout

Two-column when filter panel is open; full-width when closed.

| Column | Phase A | Phase B |
|---|---|---|
| Transcript list | Full width or ~65% | Same |
| Filter side panel | Toggled by Filter button; scrollable grouped list | Same, plus additional layer groups |

The filter panel opens **alongside** the list — both remain visible simultaneously. Closing the panel returns the list to full width.

### Source type filter

Persistent pill row above the list: `All / Scribe / Dictate / Upload`. These work on existing `HistoryRecord.source` data. No Float dependency.

### Status filter (Phase B only)

`All / Draft / Approved`. Only meaningful once Float has run on records. Hidden in Phase A.

### Filter side panel

A single scrollable grouped list. One section per layer. Sections appear in creation order (built-in layers first).

**Phase A:** Tags section only (using `HistoryRecord.tags` already stored on every record).

**Phase B:** Keywords, Decisions, and any custom layers appear as additional sections below Tags as their vocabulary accumulates via Float approval.

Each row in a section:
```
☐  item-name          N  (transcript count)
```

Checked items = active filter; list narrows to transcripts that contain any checked item within that layer. Cross-layer filtering is additive (AND between layers, OR within a layer).

Layers with zero vocabulary show: *"No vocabulary yet — approve a Float result to populate this layer."*

Footer of the panel: `N active filters · showing X of Y`.

---

## 7. Transcript detail

Unchanged from current behaviour in Phase A, except:
- Navigation path changes: back button goes to Transcripts (not History)
- Float enrichment blocks (Tags, Keywords, etc.) are **absent** in Phase A
- "Run Float flow" button is absent in Phase A

Phase B enrichment additions are covered in the Design Brain PRD.

---

## 8. Backend changes required (Phase A)

| Change | Scope |
|---|---|
| Rename History window/screen references to Dashboard | Frontend label change only |
| Expose `duration_secs` on `HistoryListItem` | Needed for "Recorded this week" stat |
| `HistoryRecord.tags` already stored | No change — filter panel reads existing data |
| New IPC command: `get_dashboard_stats` | Returns transcript count, week duration. Float-specific stats return `null` until Float ships. |

No changes to `HistoryService`, `OutputService`, or any recording pipeline.

---

## 9. Non-goals

- LLM inference, Gemma, layer extraction — Float PRD
- Layer / Step / Flow configuration — Float PRD
- Draft / Approved enrichment workflow — Float PRD
- Global search (deferred — referenced in mockup but not scoped here)
- Mobile, Linux, cloud sync

---

## 10. Open questions

| Question | Options |
|---|---|
| Float nav item in Phase A: greyed-out teaser or hidden entirely? | Teaser keeps the concept visible to early users; hiding is cleaner until ready |
| Search icon in title bar: in scope for Phase A? | Likely deferred — no search backend today |
| "Recorded this week" stat: exact week boundary (Mon–Sun vs rolling 7 days)? | ISO week (Mon) is conventional; rolling 7 days may be more useful |
| Filter panel — OR vs AND within a layer when multiple items checked? | OR within layer is the natural reading ("show me onboarding OR client-x") |

---

## 11. Reference

| Document | Purpose |
|---|---|
| [`design-brain-prd.md`](design-brain-prd.md) | Float AI processing engine — Phase B |
| [`mockup-dashboard.html`](mockup-dashboard.html) | Interactive mockup (open in browser) |
| [`history-ui-review.md`](history-ui-review.md) | Rules for History/Transcripts list and detail — do not regress |
