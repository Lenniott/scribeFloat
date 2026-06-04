# ScribeFloat — UI component catalogue

For tokens and surface layout, query `context/design-skill/design-system.json`. **History interaction rules** (fullscreen detail, delete on list only, `PanelFooter`): [../docs/history-ui-review.md](../docs/history-ui-review.md).

---

## Audio components

| Component | What it is | How it works |
|---|---|---|
| `CircularAudioVisualizer` | A parent visual component for circular live audio display.   | Composes one or more circular layers, usually mic in the centre and optional speaker as an outer overlay ring.   |
| `MicWaveRing` | A circular waveform ring for microphone input.   | Reads mic level/amplitude data and renders animated radial motion as the core waveform layer.   |
| `SpeakerWaveRing` | A circular waveform ring for speaker/system input.   | Renders a second waveform overlay; if speaker input is disabled, this component is not mounted or is hidden.   |
| `StackProgressBar` | A reusable horizontal progress indicator.   | Fills waveform-style stacked bars from left to right and shows processing steps across the row. Supports an `indeterminate` prop: when true, a scanning shimmer animation replaces the fill (used during model load when no meaningful progress is available).   |
| `AudioLayerLegend` | A small key showing which colour maps to mic and speaker.   | Displays colour swatches and labels; can optionally show disabled state for speaker when not active.   |
| `RecordingStatusDot` | Small status indicator for recording state.   | Changes appearance for idle, recording, paused, or error states.   |
| `RecordingTimer` | Elapsed-time display component.   | Shows formatted session time such as `00:00` and updates from external recording state.   |

## Form components

| Component | What it is | How it works |
|---|---|---|
| `EditableTitleField` | Inline editable text field for naming a session/file.   | Starts as plain text or input and allows direct rename without leaving the panel.   |
| `DeviceSelect` | Generic dropdown for input/output hardware selection.   | Used for things like “Selected mic” and returns the chosen device id/value.   |
| `ToggleSwitch` | Generic on/off switch component.   | Used for speaker enablement and send/export options; emits a boolean state change.   |
| `LabeledTextField` | Reusable text input with a label.   | Used for fields like mic name and speaker name when those options are active.   |
| `PathSelectorField` | Input-plus-button component for save destinations.   | Shows a path value and triggers a picker or callback when the user clicks Change.   |
| `OptionGroup` | Small grouped choice control.   | Suitable for model-size options like small and medium, using radio or segmented selection.   |

## Accordion components

The sketch clearly separates settings into collapsible groups and notes that only one can be open at a time. 

| Component | What it is | How it works |
|---|---|---|
| `Accordion` | A reusable collapsible section system.   | Controls one or more `AccordionItem` children and can enforce single-open behaviour.   |
| `AccordionItem` | One collapsible section such as Basic or Advanced.   | Has header, open/closed state, and content body; opening one can close siblings if configured.   |
| `SettingsSection` | A styled wrapper for grouped configuration controls.   | Adds layout, spacing, and optional section title inside an accordion body.   |
| `ScrollablePanel` | A constrained panel body with overflow scrolling.   | Lets long settings content scroll while the overall shell and footer stay fixed.   |

## Notes components

| Component | What it is | How it works |
|---|---|---|
| `NotesPanel` | Container for the notes side of the UI.   | Holds the notes list and note composer in a vertically structured area.   |
| `NoteCard` | A reusable note item component.   | Displays note text with its timestamp and can support selection, editing, or highlighting.   |
| `TimestampLabel` | A small metadata component for time markers.   | Shows when the note was created or when the source event occurred.   |
| `NotesList` | Scrollable list of note items.   | Renders multiple `NoteCard` components in chronological or grouped order.   |
| `NoteComposer` | Input area for adding new notes.   | Combines a text entry field and submit action for manual note creation.   |
| `IconSubmitButton` | Compact circular action button.   | Used in the note composer to submit entered content via an icon-based trigger.   |

## History components

| Component | What it is | How it works |
|---|---|---|
| `HistoryListCard` | History list row (replaces `NoteCard` in History).   | View (`Eye`), Copy, Open (when `.md` exists), Delete (store only); selectable title with hover affordance. Delete opens a confirm modal on `history.svelte`; the card emits events only.   |
| `HistoryDetailPane` | Fullscreen detail for the History view (`history.svelte`).   | Scrollable transcript via `history_render_markdown`; muted metadata from `history_get_detail`; prev/next in header; Export / Open / Copy / Close in `PanelFooter`. Delete is on the list card only.   |

## Layout components

| Component | What it is | How it works |
|---|---|---|
| `PanelShell` | Outer reusable application frame.   | Provides border, rounded container, internal layout regions, and fixed sizing.   |
| `PanelHeader` | Top bar for title, timer, and status.   | Aligns metadata and controls across the top of the shell.   |
| `PanelFooter` | Bottom bar in a flex column layout (not `position: fixed`).   | `shrink-0` sibling below a `flex-1` scroll region — actions stay visible without covering content. Used by `HistoryDetailPane`.   |
| `SplitPane` | Two-column layout wrapper.   | Places settings/audio on the left and notes on the right with a vertical divider. History no longer uses it (list/detail are separate modes).   |
| `FixedFooterBar` | Legacy footer bar component.   | Prefer `PanelFooter` for new panes.   |
| `ActionButton` | Generic footer button.   | Used for actions like Cancel and Finished, with variant styling for primary and secondary actions.   |

## Best component boundaries

For reusability, I’d separate them like this instead of tying them to “Scribe Panel” specifically. The waveform stack should be its own self-contained system, the form controls should stay generic, and notes should be independent from recording so they can be reused elsewhere. 

A clean composition would be:

- `CircularAudioVisualizer`
- `StackProgressBar`
- `Accordion`
- `DeviceSelect`
- `ToggleSwitch`
- `NotesList`
- `NoteComposer`
- `FixedFooterBar` 
