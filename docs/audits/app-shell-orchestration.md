# App Shell Orchestration Plan

> Tracks the implementation sequence for the `feature/app-shell-nav` branch.
> One agent per phase. Each phase ends with a commit and hands off to the next.
> Read this file at the start of every session — it is the source of truth for where we are.

---

## Context files to read before any session

```
CONTEXT.md                          ← canonical domain model and naming
docs/explorations/active/design-brain-prd.md ← Float direction
docs/explorations/active/knowledge-layer-intent.md ← (if exists)
docs/audits/app-shell-orchestration.md  ← this file
CLAUDE.md                           ← layer rules, build commands
```

**Naming law:** All code must use CONTEXT.md names, not the exploration branch names.
Key mapping: `history` → `note`, `dashboard` → `home`, `transcribe` (as workflow) → `upload`, `shell` → `app`, `HistoryRecord` → `Note`.

---

## Source material (exploration branch)

The `release/0.3` branch has working exploration code. Do NOT merge it — cherry-pick or manually port components. Files of interest:

| File | What to take |
|---|---|
| `src/lib/screens/app-shell.svelte` | Shell architecture, state management pattern — rename terms |
| `src/lib/components/shell/AppSidebar.svelte` | Sidebar structure — fix Scribe-as-route error, rename routes |
| `src/lib/components/shell/SidebarNavItem.svelte` | Keep as-is |
| `src/lib/components/shell/ShellTitleBar.svelte` | Keep skeleton — flesh out in phase 3 |
| `src/lib/components/settings/SettingsSidebar.svelte` | Keep — rename `shell://` → `app://` |
| `src/lib/components/settings/SettingsPanel.svelte` | Keep |
| `src/lib/components/settings/settingsTypes.ts` | Keep |
| `src/lib/screens/dashboard.svelte` | Port as `home.svelte` — rename all copy |
| `src/lib/screens/transcripts.svelte` | Port as `notes.svelte` — rename all copy |
| `src/lib/screens/capture.svelte` | Port as `capture.svelte` — fine as-is |
| `src/lib/screens/upload.svelte` | Keep stub |
| `src/lib/components/dashboard/RecentSessionCard.svelte` | Move to `components/home/` |
| `src/lib/components/dashboard/StatTile.svelte` | Move to `components/home/` |
| `src/lib/components/transcripts/TranscriptListCard.svelte` | Move to `components/notes/NoteListCard.svelte` |
| `src/lib/components/transcripts/FilterSidePanel.svelte` | Move to `components/notes/` |
| `src/lib/components/transcripts/FilterCheckboxRow.svelte` | Move to `components/notes/` |
| `src/lib/components/transcripts/SourceKindIcon.svelte` | Move to `components/notes/` |
| `src/lib/services/historyFormat.ts` | Keep — already clean |
| `src-tauri/` changes | Review per phase — tags/vocabulary backend is good |

---

## Engineering order (dependency-first)

```
Phase 1 — Foundation (no UI visible yet)
  1a. Backlog gardening (stories + archive stale) ← DONE
  1b. historyFormat.ts utility module
  1c. historyActions.ts additions (tags, TagVocabularyEntry, fetchDashboardStats)
  1d. Rust backend: add tags to HistoryListItem, add history_tag_vocabulary command

Phase 2 — Shell skeleton (replaces +page.svelte routing)
  2a. New folder structure: components/home/, components/notes/, components/shell/,
      components/settings/ (some already exist in exploration)
  2b. SidebarNavItem primitive
  2c. AppSidebar (Areas nav: Home, Notes, Upload, Float stub, Settings)
       — Scribe is NOT in the sidebar (story 0025)
       — routes: 'home' | 'notes' | 'upload' | 'float' | 'settings'
  2d. ShellTitleBar (skeleton only — "New Note" button wired in phase 3)
  2e. app-shell.svelte replaces +page.svelte as the outer layout
       — IPC event: app://navigate (not shell://navigate)
       — global state: allItems, toast, delete modal

Phase 3 — Home Area (story 0026 partial + 0024 stub)
  3a. StatTile primitive
  3b. RecentNoteCard component (was RecentSessionCard)
  3c. home.svelte screen

Phase 4 — Notes Area (story 0026 partial)
  4a. SourceKindIcon primitive
  4b. NoteListCard component (was TranscriptListCard)
  4c. FilterSidePanel + FilterCheckboxRow
  4d. notes.svelte screen (list + detail split, keep HistoryDetailPane for now)

Phase 5 — Settings sidebar pattern
  5a. settingsTypes.ts
  5b. SettingsSidebar + SettingsPanel
  5c. Wire into app-shell.svelte

Phase 6 — New Note title bar action (story 0025)
  6a. ShellTitleBar gets "New Note" button
  6b. capture.svelte / Scribe screen wired as modal or overlay from title bar
  6c. Leave guard wired through shell

Phase 7 — Rename passes (stories 0016, 0017, 0018)
  7a. shell:// events → app://
  7b. Dashboard → Home in copy and routes
  7c. Transcribe → Upload in copy, routes, kind labels

Phase 8 — Component taxonomy reorganisation (story 0027 partial)
  8a. Move files into correct folders per above source material table
  8b. Update all imports
```

---

## Handoff checkpoints

| Checkpoint | After phase | What to tell next agent |
|---|---|---|
| **HO-1** | 1d | "Backend and utility layer done. Read phases 2a–2e. Source from exploration AppSidebar + app-shell. Routes must be home/notes/upload/float/settings — not dashboard/transcripts/scribe." |
| **HO-2** | 2e | "Shell skeleton committed. Read phases 3–4. Home and Notes screens. Use CONTEXT.md names throughout. HistoryDetailPane is still the detail view — just wrap it." |
| **HO-3** | 4d | "Home + Notes Areas done. Read phase 5 (settings sidebar) then phase 6 (title bar New Note). Phase 6 removes Scribe from sidebar." |
| **HO-4** | 6c | "All areas wired. Read phases 7–8. Pure rename + file-move pass. No logic changes. Run cargo check and npm run check:ds after." |

---

## Current status

- [x] Branch created: `feature/app-shell-nav`
- [x] Backlog gardening complete (see below)
- [x] Orchestration plan written
- [x] Phase 1b — historyFormat.ts
- [x] Phase 1c — historyActions.ts additions
- [x] Phase 1d — Rust backend tags + dashboard_stats + tag_vocabulary
- [x] Phase 2 — Shell skeleton (app-shell, AppSidebar, ShellTitleBar, SidebarNavItem, capture)
- [x] Phase 3 — Home Area (home.svelte, StatTile, RecentNoteCard)
- [x] Phase 4 — Notes Area (notes.svelte, NoteListCard, FilterSidePanel)
- [x] Phase 5 — Settings sidebar (SettingsSidebar, SettingsPanel, settingsTypes)
- [x] Phase 6 — New Note title bar (story 0025) — already wired in shell
- [x] Phase 7 — Rename passes (stories 0016, 0017, 0018, 0042)
- [x] Phase 8 — Taxonomy reorganisation (story 0027)

### HO-2 handoff — for next agent

Branch: `feature/app-shell-nav`

All Areas are built and committed. The shell is wired and routing works.
The title bar already has a "New Note" button that calls `onNewNote` on the shell,
and `captureOpen` toggles the `CaptureScreen` overlay. Phase 6 is **already wired** —
test it manually before claiming it needs more work.

Phase 7 is pure renames — no logic changes:
- `shell://navigate` → `app://navigate` in Rust emitters (commands/scribe.rs, lib.rs)
- `history://item-added` → `note://item-added` in Rust and all Svelte listeners
- All `ROUTE_LABELS['dashboard']` → already done (it's `'home'` now)
- Story 0017 (Dashboard → Home in copy) — already done in home.svelte

Phase 8 is file moves:
- `components/transcripts/FilterSidePanel.svelte` → `components/notes/`
- `components/transcripts/FilterCheckboxRow.svelte` → `components/notes/`
- `components/transcripts/SourceKindIcon.svelte` → `components/notes/`
- `components/dashboard/` folder — delete (replaced by `components/home/`)
- `screens/dashboard.svelte` — delete (untracked, just rm)
- `screens/transcripts.svelte` — delete (untracked, just rm)

**Read before starting:** CONTEXT.md, CLAUDE.md, this file.

---

## Backlog stories created this session

| Story | Title |
|---|---|
| 0037 | Build app-shell.svelte — persistent sidebar shell |
| 0038 | Build Home Area screen |
| 0039 | Build Notes Area screen (list + detail) |
| 0040 | Build Settings sidebar pattern |
| 0041 | Rename HistoryListItem → NoteListItem in frontend types |
| 0042 | Rename history-related IPC events to note:// namespace |

## Stale stories archived this session

| Story | Reason |
|---|---|
| (none yet — archive after confirming against current codebase) | |
