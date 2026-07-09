# ScribeFloat — Component catalogue

For tokens and surface layout, query `skills/design-skill/design-system.json`. **Notes detail interaction rules** (fullscreen detail, delete on list only, `PanelFooter`): [../docs/history-ui-review.md](../docs/history-ui-review.md).

Components are organised into five taxonomy levels from lowest to highest composition. Folders are numbered to match the ordering, and each level has an alias + barrel file:

| Level | Folder | Alias | Example import |
|---|---|---|---|
| 1. **Primitives** | `ui/1_primitives/` | `@primitives` | `import { ScrollBody } from '@primitives/layout'` |
| 2. **Components** | `ui/2_components/` | `@components` | `import { Button, Toggle } from '@components'` |
| 3. **Patterns** | `ui/3_patterns/` | `@patterns` | `import { Accordion } from '@patterns'` |
| 4. **Sections** | `ui/4_sections/` | `@sections` | `import { FilterPanel } from '@sections'` |
| 5. **Regions** | `ui/6_regions/` | `@regions` | `import { AppSidebar } from '@regions'` |

Views live at `5_views/` → `@views`. The parent `ui/` folder is `@ui`. Other aliases: `@services`, `@stores`, `@utils` (platform, theme, types). All defined in `svelte.config.js`.

---

## Primitives

### Layout

| Component | Path | What it is |
|---|---|---|
| `ScrollBody` | `1_primitives/layout/ScrollBody.svelte` | The default body slot for pattern-B panes. `min-h-0 flex-1 overflow-y-auto overscroll-contain`. Pass padding via `class`. |
| `PanelHeader` | `1_primitives/layout/PanelHeader.svelte` | Top chrome with title + left/centre/right action slots. `shrink-0`. |
| `PanelFooter` | `1_primitives/layout/PanelFooter.svelte` | Bottom chrome for primary actions. `shrink-0`, not `position: fixed`. |
| `Modal` | `1_primitives/layout/Modal.svelte` | Focus-trap overlay with title, description, and footer snippet slots. |
| `StepFrame` | `1_primitives/layout/StepFrame.svelte` | Outer layout frame for onboarding step screens. |

### Display

| Component | Path | What it is |
|---|---|---|
| `Chip` | `1_primitives/display/Chip.svelte` | Small badge/label. Exports `ChipVariant` type (brand, focus, muted, active, warning). |
| `Timestamp` | `1_primitives/display/Timestamp.svelte` | Formatted elapsed-time label (e.g. `2:23 PM`). |
| `SourceIcon` | `1_primitives/display/SourceIcon.svelte` | Icon indicating the source kind of a note (mic, speaker, etc.). |
| `StatusDot` | `1_primitives/display/StatusDot.svelte` | Pulsing dot for recording state. Changes appearance for idle, recording, paused, error. |
| `RecordingTimer` | `1_primitives/display/RecordingTimer.svelte` | Elapsed-time display. Shows formatted session time such as `00:00` from external state. |
| `AnimatedEllipsis` | `1_primitives/display/AnimatedEllipsis.svelte` | Animated trailing dots for indeterminate waits ("Loading model…"). Inherits surrounding font/color; `aria-hidden` — the caller provides the real label text. Static under `prefers-reduced-motion`. |
| `ProgressBar` | `1_primitives/display/ProgressBar.svelte` | 2D cube-grid progress indicator. Cubes fall in with staggered timing; total grid maps to 100%. Props: `progress`, `indeterminate`, `fluid` (span parent width; columns derive from measured space), optional tuning (`rows`, `columns`, `color`, `cube`, `gap`, `scale`, `speed`). Uses `role="progressbar"` with `aria-valuenow` / `aria-valuetext`. Capture views feed it from `stores/captureProgress.svelte.ts` (monotonic, self-creeping display percent) rather than raw backend progress. |

### Form

| Component | Path | What it is |
|---|---|---|
| `TextField` | `1_primitives/form/TextField.svelte` | Text input with a label. Used for mic name, speaker name, etc. |
| `FieldRow` | `1_primitives/form/FieldRow.svelte` | Labelled config field row. Renders label + control in a consistent layout. |
| `CheckboxControl` | `1_primitives/form/CheckboxControl.svelte` | Checkbox box only (peer input + visual). Composed by `Checkbox`, `FilterRow`, etc. |
| `SettingsSection` | `1_primitives/form/SettingsSection.svelte` | Titled section container for settings screen groups. |

---

## UI Components

### Controls

| Component | Path | What it is |
|---|---|---|
| `Button` | `2_components/controls/Button.svelte` | Primary action button. Five variants: primary, destructive, ghost, normal, active. |
| `IconButton` | `2_components/controls/IconButton.svelte` | Compact icon-only button. Fewer variants than `Button` (primary, destructive, normal). |
| `Toggle` | `2_components/controls/Toggle.svelte` | On/off switch. Used for speaker enablement and export options. |
| `Checkbox` | `2_components/controls/Checkbox.svelte` | Checkbox with label. Uses `CheckboxControl` primitive. |
| `CheckboxGroup` | `2_components/controls/CheckboxGroup.svelte` | Fieldset wrapper for related checkbox rows (e.g. tag filters). |
| `EditableTitle` | `2_components/controls/EditableTitle.svelte` | Inline editable title field. Starts as plain text; switches to input on focus. |
| `PathPicker` | `2_components/controls/PathPicker.svelte` | Path value + Change button. Triggers a file-picker callback. |
| `OptionGroup` | `2_components/controls/OptionGroup.svelte` | Small grouped radio/segmented selector. Used for model size, theme, etc. |
| `MarkdownEditor` | `2_components/controls/MarkdownEditor.svelte` | CodeMirror 6 markdown editor. Props: `value` (bindable), `onchange`. Mounts a CM instance with design-token theme, line wrapping, and placeholder. No IPC — delegates saves to the parent view. |

### Cards

| Component | Path | What it is |
|---|---|---|
| `NoteCard` | `2_components/cards/NoteCard.svelte` | Notes list row. Selectable title + icon actions (Copy, View, Open, Delete). |
| `InlineNote` | `2_components/cards/InlineNote.svelte` | Inline note card. Displays note text + timestamp. Exports `Note` type. |
| `RecentNoteCard` | `2_components/cards/RecentNoteCard.svelte` | Recent-note card for the Home screen. Compact title + metadata. |
| `SettingRow` | `2_components/cards/SettingRow.svelte` | Single setting row in a settings list. Label + control slot. |
| `UploadItem` | `2_components/cards/UploadItem.svelte` | Per-item row in the upload/transcribe queue. Shows progress + status + actions. |
| `FilterRow` | `2_components/cards/FilterRow.svelte` | Checkbox row in the filter panel. Tag label + count. Uses `CheckboxControl`. |

### Nav

| Component | Path | What it is |
|---|---|---|
| `NavItem` | `2_components/nav/NavItem.svelte` | Sidebar navigation item. Icon + label + optional badge chip. Used in App and Settings sidebars. |
| `AccordionRow` | `2_components/nav/AccordionRow.svelte` | One collapsible accordion section. Must be used inside `Accordion`. |

### Indicators

| Component | Path | What it is |
|---|---|---|
| `Toast` | `2_components/indicators/Toast.svelte` | Transient notification strip. Exports `ToastState` type (normal, success, error). |
| `StatTile` | `2_components/indicators/StatTile.svelte` | Summary stat tile for the Home screen. Label + value. |
| `StepIndicator` | `2_components/indicators/StepIndicator.svelte` | Step progress dots for onboarding flows. |
| `Waveform` | `2_components/indicators/Waveform.svelte` | Live PCM stack-bar waveform visualizer. Used in Scribe + Dictate. Exports `StackBlockSize` type. |

---

## Patterns

| Component | Path | What it is |
|---|---|---|
| `Accordion` | `3_patterns/Accordion.svelte` | Collapsible section system. Controls `AccordionRow` children, enforces single-open behaviour. |
| `NoteComposer` | `3_patterns/NoteComposer.svelte` | Text entry + submit for manual note creation. |
| `NoteList` | `3_patterns/NoteList.svelte` | Scrollable list of `InlineNote` rows. |
| `UploadQueue` | `3_patterns/UploadQueue.svelte` | Scrollable list of `UploadItem` rows. |

---

## Sections

| Component | Path | What it is |
|---|---|---|
| `FilterPanel` | `4_sections/FilterPanel.svelte` | Tag-filter side panel. Vocabulary list of `FilterRow` items + active filter count. |
| `NoteDetailPane` | `4_sections/NoteDetailPane.svelte` | Fullscreen transcript + metadata detail pane (legacy + read-only store items). |
| `TranscriptPanel` | `4_sections/TranscriptPanel.svelte` | Read-only HTML transcript panel. Props: `noteId`. |
| `SettingList` | `4_sections/SettingList.svelte` | Scrollable container for `SettingRow` items. |
| `SettingsPanel` | `4_sections/SettingsPanel.svelte` | Full settings area. Routes active tab to the correct setting screen. |

### Onboarding

| Component | Path | What it is |
|---|---|---|
| `WelcomeStep` | `4_sections/onboarding/WelcomeStep.svelte` | First onboarding step: app intro + CTA. |
| `FeatureTourStep` | `4_sections/onboarding/FeatureTourStep.svelte` | Feature overview step with app menu mock. |
| `DictatePracticeStep` | `4_sections/onboarding/DictatePracticeStep.svelte` | Interactive dictation practice step. |
| `PermissionsStep` | `4_sections/onboarding/PermissionsStep.svelte` | Microphone + accessibility permission request step. |

---

## Regions

| Component | Path | What it is |
|---|---|---|
| `AppSidebar` | `6_regions/AppSidebar.svelte` | Left nav sidebar with route icons. Exports `AppRoute` type. |
| `SettingsSidebar` | `6_regions/SettingsSidebar.svelte` | Settings-mode left sidebar with tab nav. Back navigation is handled by `TitleBar`. |
| `TitleBar` | `6_regions/TitleBar.svelte` | Top title bar chrome. Optional back button (note editor, settings), Record, scribe controls (waveform, timer, settings gear), and Dictate. State from `scribeController` store. |

---

## Views

Route-level view components. Imported by `src/routes/` pages. Live at `src/lib/ui/5_views/` → `@views`.

| View | Path | What it is |
|---|---|---|
| `home.svelte` | `5_views/home.svelte` | Home area: recent notes + stats. |
| `notes.svelte` | `5_views/notes.svelte` | Notes list + detail pane. |
| `note-editor.svelte` | `5_views/note-editor.svelte` | Unified note editor at `/notes/[id]`. Written panel (always) + optional transcript/metadata side panels. `TranscriptPanel`. Recording is controlled from `TitleBar` via `scribeController`. Autosave dirty-checks before IPC; body/title persist to `.notes/{id}/` sidecars (see `docs/engineering/history-storage.md`). |
| `capture.svelte` | `5_views/capture.svelte` | Scribe capture overlay (recording → processing). |
| `transcribe.svelte` | `5_views/transcribe.svelte` | Upload/transcribe workflow. |
| `dictate.svelte` | `5_views/dictate.svelte` | Floating Dictate HUD (separate Tauri window). |
| `onboarding.svelte` | `5_views/onboarding.svelte` | Onboarding flow (separate Tauri window). |
| `setting_*.svelte` | `5_views/setting_*.svelte` | Individual settings tab content (general, advanced, voice, permissions, help). |
