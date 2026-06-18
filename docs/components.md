# ScribeFloat — Component catalogue

For tokens and surface layout, query `skills/design-skill/design-system.json`. **History interaction rules** (fullscreen detail, delete on list only, `PanelFooter`): [../docs/history-ui-review.md](../docs/history-ui-review.md).

Components are organised into five taxonomy levels from lowest to highest composition:

1. **Primitives** — atoms with no internal component dependencies
2. **UI** — named interactive units built from primitives
3. **Patterns** — repeated multi-step flows or data presentations
4. **Sections** — full screen sections, assembled from components above
5. **Regions** — persistent shell zones that frame the entire app

---

## Primitives

### Layout

| Component | Path | What it is |
|---|---|---|
| `ScrollBody` | `primitives/layout/ScrollBody.svelte` | The default body slot for pattern-B panes. `min-h-0 flex-1 overflow-y-auto overscroll-contain`. Pass padding via `class`. |
| `PanelHeader` | `primitives/layout/PanelHeader.svelte` | Top chrome with title + left/centre/right action slots. `shrink-0`. |
| `PanelFooter` | `primitives/layout/PanelFooter.svelte` | Bottom chrome for primary actions. `shrink-0`, not `position: fixed`. |
| `Modal` | `primitives/layout/Modal.svelte` | Focus-trap overlay with title, description, and footer snippet slots. |
| `StepFrame` | `primitives/layout/StepFrame.svelte` | Outer layout frame for onboarding step screens. |

### Display

| Component | Path | What it is |
|---|---|---|
| `Chip` | `primitives/display/Chip.svelte` | Small badge/label. Exports `ChipVariant` type (brand, focus, muted, active, warning). |
| `Timestamp` | `primitives/display/Timestamp.svelte` | Formatted elapsed-time label (e.g. `2:23 PM`). |
| `SourceIcon` | `primitives/display/SourceIcon.svelte` | Icon indicating the source kind of a note (mic, speaker, etc.). |
| `StatusDot` | `primitives/display/StatusDot.svelte` | Pulsing dot for recording state. Changes appearance for idle, recording, paused, error. |
| `RecordingTimer` | `primitives/display/RecordingTimer.svelte` | Elapsed-time display. Shows formatted session time such as `00:00` from external state. |
| `ProgressBar` | `primitives/display/ProgressBar.svelte` | Horizontal stacked-bar progress indicator. Supports `indeterminate` shimmer mode. |

### Form

| Component | Path | What it is |
|---|---|---|
| `TextField` | `primitives/form/TextField.svelte` | Text input with a label. Used for mic name, speaker name, etc. |
| `FieldRow` | `primitives/form/FieldRow.svelte` | Labelled config field row. Renders label + control in a consistent layout. |
| `Checkbox` | `primitives/form/Checkbox.svelte` | Standard checkbox with accessible label. |
| `SettingsSection` | `primitives/form/SettingsSection.svelte` | Titled section container for settings screen groups. |

---

## UI Components

### Controls

| Component | Path | What it is |
|---|---|---|
| `Button` | `ui/controls/Button.svelte` | Primary action button. Five variants: primary, destructive, ghost, normal, active. |
| `IconButton` | `ui/controls/IconButton.svelte` | Compact icon-only button. Fewer variants than `Button` (primary, destructive, normal). |
| `Toggle` | `ui/controls/Toggle.svelte` | On/off switch. Used for speaker enablement and export options. |
| `EditableTitle` | `ui/controls/EditableTitle.svelte` | Inline editable title field. Starts as plain text; switches to input on focus. |
| `PathPicker` | `ui/controls/PathPicker.svelte` | Path value + Change button. Triggers a file-picker callback. |
| `OptionGroup` | `ui/controls/OptionGroup.svelte` | Small grouped radio/segmented selector. Used for model size, theme, etc. |

### Cards

| Component | Path | What it is |
|---|---|---|
| `NoteCard` | `ui/cards/NoteCard.svelte` | History list row (was `NoteListCard`). Selectable title + icon actions (Copy, View, Open, Delete). |
| `InlineNote` | `ui/cards/InlineNote.svelte` | Inline note card. Displays note text + timestamp. Exports `Note` type. |
| `RecentNoteCard` | `ui/cards/RecentNoteCard.svelte` | Recent-note card for the Home screen. Compact title + metadata. |
| `SettingRow` | `ui/cards/SettingRow.svelte` | Single setting row in a settings list. Label + control slot. |
| `UploadItem` | `ui/cards/UploadItem.svelte` | Per-item row in the upload/transcribe queue. Shows progress + status + actions. |
| `FilterRow` | `ui/cards/FilterRow.svelte` | Checkbox row in the filter panel. Tag label + checkbox. |

### Nav

| Component | Path | What it is |
|---|---|---|
| `NavButton` | `ui/nav/NavButton.svelte` | Route navigation button. Used for top-level app navigation. |
| `NavItem` | `ui/nav/NavItem.svelte` | Sidebar navigation item. Icon + label + optional badge chip. |
| `AccordionRow` | `ui/nav/AccordionRow.svelte` | One collapsible accordion section. Must be used inside `Accordion`. |

### Indicators

| Component | Path | What it is |
|---|---|---|
| `Toast` | `ui/indicators/Toast.svelte` | Transient notification strip. Exports `ToastState` type (normal, success, error). |
| `StatTile` | `ui/indicators/StatTile.svelte` | Summary stat tile for the Home screen. Label + value. |
| `StepIndicator` | `ui/indicators/StepIndicator.svelte` | Step progress dots for onboarding flows. |
| `Waveform` | `ui/indicators/Waveform.svelte` | Live PCM stack-bar waveform visualizer. Used in Scribe + Dictate. Exports `StackBlockSize` type. |

---

## Patterns

| Component | Path | What it is |
|---|---|---|
| `Accordion` | `patterns/Accordion.svelte` | Collapsible section system. Controls `AccordionRow` children, enforces single-open behaviour. |
| `NoteComposer` | `patterns/NoteComposer.svelte` | Text entry + submit for manual note creation. |
| `NoteList` | `patterns/NoteList.svelte` | Scrollable list of `InlineNote` rows. |
| `UploadQueue` | `patterns/UploadQueue.svelte` | Scrollable list of `UploadItem` rows. |

---

## Sections

| Component | Path | What it is |
|---|---|---|
| `FilterPanel` | `sections/FilterPanel.svelte` | Tag-filter side panel. Vocabulary list of `FilterRow` items + active filter count. |
| `NoteDetailPane` | `sections/NoteDetailPane.svelte` | Fullscreen transcript + metadata detail pane (was `HistoryDetailPane`). |
| `SettingList` | `sections/SettingList.svelte` | Scrollable container for `SettingRow` items. |
| `SettingsPanel` | `sections/SettingsPanel.svelte` | Full settings area. Routes active tab to the correct setting screen. |

### Onboarding

| Component | Path | What it is |
|---|---|---|
| `WelcomeStep` | `sections/onboarding/WelcomeStep.svelte` | First onboarding step: app intro + CTA. |
| `FeatureTourStep` | `sections/onboarding/FeatureTourStep.svelte` | Feature overview step with app menu mock. |
| `DictatePracticeStep` | `sections/onboarding/DictatePracticeStep.svelte` | Interactive dictation practice step. |
| `PermissionsStep` | `sections/onboarding/PermissionsStep.svelte` | Microphone + accessibility permission request step. |
| `ModelDownloadStep` | `sections/onboarding/ModelDownloadStep.svelte` | Whisper model download step with progress. |

---

## Regions

| Component | Path | What it is |
|---|---|---|
| `AppSidebar` | `regions/AppSidebar.svelte` | Left nav sidebar with route icons. Exports `AppRoute` type. |
| `SettingsSidebar` | `regions/SettingsSidebar.svelte` | Settings-mode left sidebar with tab nav and back button. |
| `TitleBar` | `regions/TitleBar.svelte` | Top title bar chrome. Houses the New Note button and recording state indicator. |
