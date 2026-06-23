<script lang="ts">
  import Accordion from "@patterns/Accordion.svelte";
  import AccordionItem from "@components/nav/AccordionRow.svelte";
  import SettingsSection from "@primitives/form/SettingsSection.svelte";
  import Chip from "@primitives/display/Chip.svelte";
  import SourceIcon from "@primitives/display/SourceIcon.svelte";
  import RecordingStatusDot from "@primitives/display/StatusDot.svelte";
  import RecordingTimer from "@primitives/display/RecordingTimer.svelte";
  import PanelHeader from "@primitives/layout/PanelHeader.svelte";
  import PanelFooter from "@primitives/layout/PanelFooter.svelte";
  import ScrollBody from "@primitives/layout/ScrollBody.svelte";
  import Modal from "@primitives/layout/Modal.svelte";
  import StepFrame from "@primitives/layout/StepFrame.svelte";
  import Button from "@components/controls/Button.svelte";
  import IconButton from "@components/controls/IconButton.svelte";
  import Checkbox from "@components/controls/Checkbox.svelte";
  import CheckboxGroup from "@components/controls/CheckboxGroup.svelte";
  import ConfigField from "@primitives/form/FieldRow.svelte";
  import EditableTitleField from "@components/controls/EditableTitle.svelte";
  import LabeledTextField from "@primitives/form/TextField.svelte";
  import OptionGroup from "@components/controls/OptionGroup.svelte";
  import PathPicker from "@components/controls/PathPicker.svelte";
  import StackProgressBar from "@primitives/display/ProgressBar.svelte";
  import ToggleSwitch from "@components/controls/Toggle.svelte";
  import InlineNoteCard from "@components/cards/InlineNote.svelte";
  import HistoryNoteCard from "@components/cards/NoteCard.svelte";
  import RecentNoteCard from "@components/cards/RecentNoteCard.svelte";
  import SettingRow from "@components/cards/SettingRow.svelte";
  import FilterRow from "@components/cards/FilterRow.svelte";
  import UploadItem from "@components/cards/UploadItem.svelte";
  import NavItem from "@components/nav/NavItem.svelte";
  import Toast from "@components/indicators/Toast.svelte";
  import StatTile from "@components/indicators/StatTile.svelte";
  import StepIndicator from "@components/indicators/StepIndicator.svelte";
  import NoteComposer from "@patterns/NoteComposer.svelte";
  import NotesList from "@patterns/NoteList.svelte";
  import UploadQueue from "@patterns/UploadQueue.svelte";
  import SettingList from "@sections/SettingList.svelte";
  import FilterPanel from "@sections/FilterPanel.svelte";
  import NoteDetailPane from "@sections/NoteDetailPane.svelte";
  import WelcomeStep from "@sections/onboarding/WelcomeStep.svelte";
  import FeatureTourStep from "@sections/onboarding/FeatureTourStep.svelte";
  import PermissionsStep from "@sections/onboarding/PermissionsStep.svelte";
  import ModelDownloadStep from "@sections/onboarding/ModelDownloadStep.svelte";
  import DictatePracticeStep from "@sections/onboarding/DictatePracticeStep.svelte";
  import AppSidebar, { type AppRoute } from "@regions/AppSidebar.svelte";
  import SettingsSidebar from "@regions/SettingsSidebar.svelte";
  import TitleBar from "@regions/TitleBar.svelte";
  import TimestampLabel from "@primitives/display/Timestamp.svelte";
  import type { Note } from "@components/cards/InlineNote.svelte";
  import type { HistoryListItem } from "@services/historyActions";
  import type { SettingsTab } from "@sections/settingsTypes";
  import type { TranscribeQueueItemView } from "@components/cards/UploadItem.svelte";
  import { applyThemeMode, type ThemeMode } from '@utils/theme';
  import { X as Close } from "lucide-svelte";
  import { FileText, Home, Settings } from "lucide-svelte";
  import ChevronRight from "lucide-svelte/icons/chevron-right";
  import Plus from "lucide-svelte/icons/plus";
  import Trash2 from "lucide-svelte/icons/trash-2";
  import AudioWaveFormVisualizer from "@components/indicators/Waveform.svelte";

  /** Local playground state — only for exercising controls, not a real screen */
  let toggleA = $state(false);
  let checkboxA = $state(true);
  let selectValue = $state("a");
  let textA = $state("");
  let titleDemo = $state("Editable title");
  let hotkeyDemo = $state("Cmd+Shift+H");
  let optionDemo = $state("one");
  let notesDemo = $state<Note[]>([
    { id: "n1", text: "Example note body.", recordedAtMs: 120_000 },
  ]);
  let selectedNoteId = $state<string | null>(null);
  let draft = $state("");
  let previewTheme = $state<ThemeMode>("system");
  let modalOpen = $state(false);
  let toastMessage = $state("");
  let toastState = $state<"normal" | "success" | "error">("normal");
  let demoRoute = $state<AppRoute>("home");
  let demoSettingsTab = $state<SettingsTab>("general");
  let filterTags = $state(new Set<string>(["meeting"]));
  let showNoteDetail = $state(false);
  let pathPickerDemo = $state("~/Documents/ScribeFloat");

  const sourceIconKinds = [
    { kind: "dictate", label: "Dictate" },
    { kind: "transcribe", label: "Upload" },
    { kind: "scribe", label: "Scribe" },
  ] as const;

  const catalogSections = [
    { id: "sec-theme", label: "Theme modes" },
    { id: "sec-colors", label: "Color roles" },
    { id: "sec-type", label: "Typography" },
    { id: "sec-geo", label: "Geometry" },
    { id: "sec-display", label: "Primitives · display" },
    { id: "sec-layout", label: "Primitives · layout" },
    { id: "sec-buttons", label: "Button" },
    { id: "sec-icon-buttons", label: "IconButton" },
    { id: "sec-forms", label: "Form" },
    { id: "sec-acc", label: "Accordion" },
    { id: "sec-nav", label: "Nav" },
    { id: "sec-indicators", label: "Indicators" },
    { id: "sec-cards", label: "Cards" },
    { id: "sec-patterns", label: "Patterns" },
    { id: "sec-sections", label: "Sections" },
    { id: "sec-regions", label: "Regions" },
    { id: "sec-onboarding", label: "Onboarding" },
    { id: "sec-prototypes", label: "Prototypes" },
    { id: "sec-audio", label: "Audio" },
    { id: "sec-legacy-notes", label: "Legacy note UI" },
  ] as const;

  const demoNoteItem: HistoryListItem = {
    id: "demo-note-1",
    kind: "dictate",
    created_at: new Date().toISOString(),
    title: "Team standup",
    model: "base",
    word_count: 342,
    duration_ms: 180_000,
    duration_secs: 180,
    excerpt: "Discussed sprint priorities and blockers for the release.",
    tags: ["work", "meeting"],
    has_markdown: true,
    markdown_path: "/tmp/demo.md",
    source: "store",
  };

  const demoLegacyNoteItem: HistoryListItem = {
    ...demoNoteItem,
    id: "demo-note-2",
    kind: "scribe",
    title: "Legacy import",
    source: "legacy",
    has_markdown: false,
    tags: ["archive"],
  };

  const demoQueueItems: TranscribeQueueItemView[] = [
    {
      id: "q1",
      source_path: "/Users/demo/interview.wav",
      display_name: "interview.wav",
      source_type: "single_audio",
      duration_ms: 312_000,
      status: "DONE",
      progress: 1,
      transcript_path: "/tmp/interview.md",
    },
    {
      id: "q2",
      source_path: "/Users/demo/meeting-dual",
      display_name: "meeting-dual",
      source_type: "dual_source_session",
      duration_ms: 540_000,
      status: "PROCESSING",
      progress: 0.42,
    },
    {
      id: "q3",
      source_path: "/Users/demo/broken.m4a",
      display_name: "broken.m4a",
      source_type: "single_audio",
      duration_ms: 45_000,
      status: "ERROR",
      progress: 0,
      error: "Could not decode audio stream.",
    },
  ];

  const filterVocabulary = [
    { name: "meeting", count: 12 },
    { name: "work", count: 8 },
    { name: "draft", count: 3 },
  ];

  const selectOptions = [
    { value: "a", label: "Option A" },
    { value: "b", label: "Option B" },
  ];

  function appendDemoNote(text: string) {
    notesDemo = [
      ...notesDemo,
      { id: crypto.randomUUID(), text, recordedAtMs: 45_000 },
    ];
  }

  function onComposerDone() {
    const t = draft.trim();
    if (!t) return;
    appendDemoNote(t);
    draft = "";
  }

  function showToastDemo(message: string, state: "normal" | "success" | "error" = "normal") {
    toastMessage = message;
    toastState = state;
    setTimeout(() => {
      toastMessage = "";
    }, 2500);
  }

  const recordingStatuses = ["idle", "recording", "paused", "error"] as const;

  /** Every `--color-*` from `app.css` @theme (shown as bg-* swatches) */
  const colorTokens: { token: string; class: string }[] = [
    { token: "canvas", class: "bg-canvas" },
    { token: "panel", class: "bg-panel" },
    { token: "card", class: "bg-card" },
    { token: "fill", class: "bg-fill" },
    { token: "rim", class: "bg-rim" },
    { token: "fg", class: "bg-fg" },
    { token: "fg-dim", class: "bg-fg-dim" },
    { token: "fg-muted", class: "bg-fg-muted" },
    { token: "brand", class: "bg-brand" },
    { token: "on-brand", class: "bg-on-brand" },
    { token: "brand-hover", class: "bg-brand-hover" },
    { token: "on-brand-hover", class: "bg-on-brand-hover" },
    { token: "warning", class: "bg-warning" },
    { token: "on-warning", class: "bg-on-warning" },
    { token: "active", class: "bg-active" },
    { token: "on-active", class: "bg-on-active" },
    { token: "destructive", class: "bg-destructive" },
    { token: "destructive-hover", class: "bg-destructive-hover" },
    { token: "on-destructive", class: "bg-on-destructive" },
    { token: "success", class: "bg-success" },
    { token: "on-success", class: "bg-on-success" },
    { token: "focus", class: "bg-focus" },
  ];

  const variants = [
    "primary",
    "destructive",
    "ghost",
    "normal",
    "active",
  ] as const;
  /** IconButton intentionally supports fewer variants than Button */
  const iconButtonVariants = ["primary", "destructive", "normal"] as const;
  const sizes = ["normal", "small"] as const;
  const stackProgressSequence = [
    { label: "Loading model", complete: true },
    { label: "Transcribing audio", complete: true },
    { label: "Writing transcript", complete: false },
    { label: "Cleaning up audio", complete: false },
  ];

  /** Static demo elapsed time for recording-bar prototype (14:07) */
  const prototypeElapsedSeconds = 847;
  const prototypeMicLevel = 0.52;
  const prototypeSpeakerLevel = 0.28;

  /** Matches shipped dictate.svelte recording HUD — reuse in all prototype mocks */
  const dictateHudClass =
    "flex w-60 items-center justify-between gap-2 rounded-md bg-panel py-2 pl-3 pr-2 shadow-ambient";

  const scribeBarProtoClass =
    "scribe-recording-bar-proto flex h-9 w-full items-center gap-3 border-b-2 border-destructive px-2 text-fg";

  const themeOptions = [
    { value: "system", label: "System" },
    { value: "dark", label: "Dark" },
    { value: "light", label: "Light" },
  ];

  $effect(() => {
    applyThemeMode(previewTheme);
  });
</script>

<main class="mx-auto flex h-full min-h-0 flex-col overflow-y-auto bg-canvas text-left p-4">
  <a href="/" class="sf-label-md text-brand hover:text-brand-hover">Home</a>
  <header class="mb-14 max-w-2xl">
    <p class="sf-label-sm text-fg-dim">
      ScribeFloat · design system
    </p>
    <h1 class="sf-display-lg text-fg">
      Design system
    </h1>
    <p class="mt-3 sf-body-md text-fg-dim leading-relaxed">
      Canonical UI for ScribeFloat — tokens, <code class="text-brand">sf-*</code> typography
      roles, and components organised by taxonomy level (Primitive, Component, Pattern,
      Section, Region). Domain terms follow <code class="text-brand">CONTEXT.md</code>.
    </p>
    <nav class="mt-8 rounded-md border border-rim bg-card p-4" aria-label="Component catalog">
      <p class="sf-section-label text-fg-dim mb-3">
        Catalog
      </p>
      <ul class="flex flex-wrap gap-x-4 gap-y-2">
        {#each catalogSections as { id, label } (id)}
          <li>
            <a
              href={"#" + id}
              class="sf-label-md text-brand hover:text-brand-hover underline-offset-2 hover:underline"
            >
              {label}
            </a>
          </li>
        {/each}
      </ul>
    </nav>
  </header>

  <section class="mb-16" aria-labelledby="sec-theme">
    <h2
      id="sec-theme"
      class="mb-6 sf-headline-sm text-fg"
    >
      Theme Modes
    </h2>
    <div class="max-w-md rounded-md bg-card p-6">
      <OptionGroup
        name="theme-preview"
        label="Preview theme"
        options={themeOptions}
        bind:selected={previewTheme}
      />
      <p class="mt-4 sf-body-md text-fg-dim">
        The app stores <code class="text-brand">system</code>,
        <code class="text-brand">dark</code>, or
        <code class="text-brand">light</code> in settings and resolves those to document-level
        theme tokens.
      </p>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-colors">
    <h2
      id="sec-colors"
      class="mb-6 sf-headline-sm text-fg"
    >
      Color Roles
    </h2>
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
      {#each colorTokens as { token, class: c } (token)}
        <div class="flex flex-col gap-2">
          <div class="rounded-md bg-card p-2">
            <div class="h-12 rounded-md {c}"></div>
          </div>
          <span class="sf-label-sm text-fg"
            >{token}</span
          >
        </div>
      {/each}
    </div>
    <p class="sf-label-sm mt-4 text-fg-dim">
      Utilities mirror tokens — use <code class="text-fg">bg-*</code>,
      <code class="text-fg">text-*</code>, or
      <code class="text-fg">border-*</code> with the same name (for example
      <code class="text-fg">border-active</code>).
    </p>
  </section>

  <section class="mb-16" aria-labelledby="sec-type">
    <h2
      id="sec-type"
      class="mb-6 sf-headline-sm text-fg"
    >
      Typography
    </h2>
    <div class="flex flex-col gap-6 bg-card p-6 rounded-md">
      <div>
        <p class="sf-section-label text-fg-dim mb-1">sf-display-lg</p>
        <p class="sf-display-lg text-fg">Record clearly</p>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-1">sf-headline-sm</p>
        <p class="sf-headline-sm text-fg">Section header</p>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-1">sf-section-label</p>
        <p class="sf-section-label text-fg-dim">Transcript · input</p>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-1">sf-body-md</p>
        <p class="sf-body-md text-fg">
          Standard UI copy defaults to light weight with relaxed leading for dense layouts.
        </p>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-1">sf-label-sm / sf-label-md</p>
        <p class="sf-label-sm text-fg-dim">Metadata</p>
        <p class="sf-label-md text-fg-dim">Secondary label</p>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-1">sf-meta-sm</p>
        <p class="sf-meta-sm text-fg-dim">2:23 PM · 14:07</p>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-geo">
    <h2
      id="sec-geo"
      class="mb-6 sf-headline-sm text-fg"
    >
      Geometry
    </h2>
    <div class="flex flex-wrap items-end gap-6">
      <div class="flex flex-col gap-2">
        <span class="sf-label-sm text-fg-dim">radius-md (4px)</span>
        <div class="h-16 w-16 rounded-md bg-card"></div>
      </div>
      <div class="flex flex-col gap-2">
        <span class="sf-label-sm text-fg-dim">radius-sm (2px)</span>
        <div class="h-16 w-16 rounded-sm bg-card"></div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-display">
    <h2
      id="sec-display"
      class="mb-6 sf-headline-sm text-fg"
    >
      Primitives · display
    </h2>
    <div class="flex flex-col gap-8 max-w-2xl">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">Chip</p>
        <div class="flex flex-wrap gap-2">
          <Chip variant="brand">Brand</Chip>
          <Chip variant="focus">Focus</Chip>
          <Chip variant="muted">Muted</Chip>
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">SourceIcon</p>
        <p class="sf-body-md text-fg-muted mb-3">
          Capture-method icons. <code class="text-brand">transcribe</code> kind maps to the
          Upload Area in product copy.
        </p>
        <div class="flex flex-wrap gap-4">
          {#each sourceIconKinds as { kind, label } (kind)}
            <div class="flex flex-col items-center gap-2">
              <SourceIcon {kind} />
              <span class="sf-label-md text-fg-dim">{label}</span>
            </div>
          {/each}
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">Timestamp</p>
        <TimestampLabel at={94_000} />
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-layout">
    <h2
      id="sec-layout"
      class="mb-6 sf-headline-sm text-fg"
    >
      Primitives · layout
    </h2>
    <div class="flex flex-col gap-10 max-w-2xl">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">PanelHeader + ScrollBody + PanelFooter</p>
        <div class="flex h-64 flex-col overflow-hidden rounded-md border border-rim bg-panel">
          <PanelHeader>
            {#snippet left()}
              <p class="sf-headline-sm text-fg">Pane title</p>
            {/snippet}
            {#snippet right()}
              <Button variant="normal" size="small">Action</Button>
            {/snippet}
          </PanelHeader>
          <ScrollBody class="px-4 py-3">
            <p class="sf-body-md text-fg-dim">
              Scrollable body content. Chrome stays fixed above and below.
            </p>
            {#each Array(8) as _, i (i)}
              <p class="sf-body-md text-fg mt-2">Line {i + 1}</p>
            {/each}
          </ScrollBody>
          <PanelFooter>
            <Button variant="ghost" size="small">Cancel</Button>
            <Button variant="primary" size="small">Save</Button>
          </PanelFooter>
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">Modal</p>
        <Button variant="normal" onclick={() => (modalOpen = true)}>Open modal</Button>
        <Modal
          open={modalOpen}
          title="Example modal"
          description="Focus-trapped overlay with footer actions."
          onClose={() => (modalOpen = false)}
        >
          <p class="sf-body-md text-fg-dim">Modal body content goes here.</p>
          {#snippet footer()}
            <Button variant="ghost" size="small" onclick={() => (modalOpen = false)}>
              Cancel
            </Button>
            <Button variant="primary" size="small" onclick={() => (modalOpen = false)}>
              Confirm
            </Button>
          {/snippet}
        </Modal>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">StepFrame</p>
        <div class="h-48 rounded-md border border-rim bg-panel p-6">
          <StepFrame title="Step title" subtitle="Optional subtitle for onboarding steps.">
            {#snippet children()}
              <p class="sf-body-md text-fg-dim">Step body slot.</p>
            {/snippet}
            {#snippet footer()}
              <Button variant="ghost" size="small">Back</Button>
              <Button variant="primary" size="small">Continue</Button>
            {/snippet}
          </StepFrame>
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-buttons">
    <h2
      id="sec-buttons"
      class="mb-6 sf-headline-sm text-fg"
    >
      Button
    </h2>
    <div class="flex flex-col gap-8">
      {#each sizes as size (size)}
        <div>
          <p class="sf-section-label text-fg-dim mb-3">
            Size · {size}
          </p>
          <div class="flex flex-wrap gap-3">
            {#each variants as v (v)}
              <Button variant={v} {size}>{v}</Button>
            {/each}
          </div>
        </div>
      {/each}
      <div>
        <p class="sf-section-label text-fg-dim mb-3">With icon</p>
        <div class="flex flex-wrap gap-3">
          <Button variant="primary" icon={ChevronRight}>Next</Button>
          <Button variant="normal" size="small" icon={Plus}>Add</Button>
          <Button variant="destructive" icon={Trash2}>Remove</Button>
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-icon-buttons">
    <h2
      id="sec-icon-buttons"
      class="mb-6 sf-headline-sm text-fg"
    >
      IconButton
    </h2>
    <p class="sf-body-md text-fg-dim mb-6 max-w-xl">
      Icon-only control. Requires <code class="text-brand">aria-label</code>.
      Variants:
      <code class="text-brand">primary</code>,
      <code class="text-brand">destructive</code>,
      <code class="text-brand">normal</code>.
    </p>
    <div class="flex flex-col gap-8">
      {#each sizes as size (size)}
        <div>
          <p class="sf-section-label text-fg-dim mb-3">
            Size · {size}
          </p>
          <div class="flex flex-wrap items-center gap-3">
            {#each iconButtonVariants as v (v)}
              <IconButton
                variant={v}
                {size}
                icon={v === "destructive" ? Trash2 : Plus}
                aria-label="{v} icon button"
              />
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-forms">
    <h2
      id="sec-forms"
      class="mb-6 sf-headline-sm text-fg"
    >
      Form
    </h2>
    <div
      class="flex max-w-md flex-col gap-8 bg-card p-6 rounded-md"
    >
      <div class="flex flex-col gap-4">
        <p class="sf-section-label text-fg-dim">Text &amp; choice</p>
      <ConfigField
        label="ConfigField (select)"
        mode="select"
        options={selectOptions}
        bind:value={selectValue}
      />
      <LabeledTextField
        label="LabeledTextField"
        bind:value={textA}
        placeholder="Placeholder"
      />
      <div class="flex items-center justify-between gap-4">
        <ToggleSwitch label="Demo toggle" bind:checked={toggleA} />
      </div>
      <Checkbox label="Demo checkbox" bind:checked={checkboxA} />
      <OptionGroup
        name="ds-option"
        label="OptionGroup"
        options={[
          { value: "one", label: "One" },
          { value: "two", label: "Two" },
        ]}
        bind:selected={optionDemo}
      />
      </div>
      <div class="flex flex-col gap-4">
        <p class="sf-section-label text-fg-dim">Value + action button</p>
      <ConfigField
        label="ConfigField (hotkey)"
        mode="action"
        bind:value={hotkeyDemo}
        buttonLabel="Capture"
        onButtonClick={() => {}}
      />
      <PathPicker label="PathPicker" bind:path={pathPickerDemo} />
      </div>
      <div class="flex flex-col gap-4">
        <p class="sf-section-label text-fg-dim">Page title (headline typography, not a form field)</p>
      <EditableTitleField bind:value={titleDemo} />
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-acc">
    <h2
      id="sec-acc"
      class="mb-6 sf-headline-sm text-fg"
    >
      Accordion
    </h2>
    <div class="max-w-md">
      <Accordion>
        <AccordionItem id="ds-1" title="First section">
          <SettingsSection title="Inner title">
            <p class="sf-body-md text-fg-dim">
              SettingsSection + AccordionItem body.
            </p>
          </SettingsSection>
        </AccordionItem>
        <AccordionItem id="ds-2" title="Second section">
          <p class="sf-body-md text-fg-dim">Another panel.</p>
        </AccordionItem>
      </Accordion>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-nav">
    <h2
      id="sec-nav"
      class="mb-6 sf-headline-sm text-fg"
    >
      Nav
    </h2>
    <div class="flex flex-col gap-8 max-w-xs">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">NavItem</p>
        <div class="flex flex-col gap-1 rounded-md border border-rim bg-panel p-2">
          <NavItem label="Home" icon={Home} active onclick={() => {}} />
          <NavItem label="Notes" icon={FileText} onclick={() => {}} />
          <NavItem label="General" icon={Settings} active onclick={() => {}} />
          <NavItem label="Float" icon={Home} disabled badge="Coming soon" />
          <NavItem label="Settings" icon={Settings} accent onclick={() => {}} />
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-indicators">
    <h2
      id="sec-indicators"
      class="mb-6 sf-headline-sm text-fg"
    >
      Indicators
    </h2>
    <div class="flex flex-col gap-8 max-w-2xl">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">StatTile</p>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <StatTile value="24" label="Notes" />
          <StatTile value="3h 12m" label="Recorded this week" highlight />
          <StatTile value="—" label="Layers" />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">StepIndicator</p>
        <div class="flex flex-wrap gap-6">
          {#each [2, 3, 4, 5] as step (step)}
            <div class="flex flex-col items-center gap-2">
              <StepIndicator currentStep={step} />
              <span class="sf-label-md text-fg-dim">step {step}</span>
            </div>
          {/each}
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">Toast</p>
        <div class="flex flex-wrap gap-2">
          <Button variant="normal" size="small" onclick={() => showToastDemo("Saved", "success")}>
            Success
          </Button>
          <Button variant="normal" size="small" onclick={() => showToastDemo("Copy failed", "error")}>
            Error
          </Button>
          <Button variant="normal" size="small" onclick={() => showToastDemo("Processing…", "normal")}>
            Normal
          </Button>
        </div>
        <Toast message={toastMessage} state={toastState} position="bottom-center" />
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-cards">
    <h2
      id="sec-cards"
      class="mb-6 sf-headline-sm text-fg"
    >
      Cards
    </h2>
    <div class="flex max-w-2xl flex-col gap-8">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">NoteCard — Notes Area list row</p>
        <HistoryNoteCard
          item={demoNoteItem}
          chip={{ label: "Approved", variant: "brand" }}
          onselect={() => {}}
          oncopy={() => {}}
          onopen={() => {}}
          ondelete={() => {}}
        />
        <div class="mt-3">
          <HistoryNoteCard
            item={demoLegacyNoteItem}
            chip={{ label: "Pending", variant: "muted" }}
          />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">RecentNoteCard — Home Area</p>
        <RecentNoteCard item={demoNoteItem} onselect={() => {}} />
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">SettingRow</p>
        <SettingList>
          <SettingRow title="Speaker capture" description="Record system audio alongside the mic.">
            {#snippet control()}
              <ToggleSwitch aria-label="Speaker capture" bind:checked={toggleA} />
            {/snippet}
          </SettingRow>
          <SettingRow title="Export format" description="Markdown is the default export.">
            {#snippet children()}
              <OptionGroup
                name="ds-export"
                label="Export format"
                labelHidden
                options={[
                  { value: "md", label: "Markdown" },
                  { value: "txt", label: "Plain text" },
                ]}
                bind:selected={optionDemo}
              />
            {/snippet}
          </SettingRow>
        </SettingList>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">FilterRow + CheckboxGroup</p>
        <div class="max-w-xs rounded-md border border-rim bg-panel p-2">
          <CheckboxGroup>
            <FilterRow label="meeting" count={12} checked={true} onchange={() => {}} />
            <FilterRow label="work" count={8} checked={false} onchange={() => {}} />
          </CheckboxGroup>
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">UploadItem</p>
        <div class="rounded-md border border-rim bg-panel">
          <UploadItem index={0} item={demoQueueItems[1]} onRemove={() => {}} />
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-patterns">
    <h2
      id="sec-patterns"
      class="mb-6 sf-headline-sm text-fg"
    >
      Patterns
    </h2>
    <div class="max-w-2xl">
      <p class="sf-section-label text-fg-dim mb-3">UploadQueue — Upload Area</p>
      <UploadQueue items={demoQueueItems} onRemove={() => {}} onOpenTranscript={() => {}} />
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-sections">
    <h2
      id="sec-sections"
      class="mb-6 sf-headline-sm text-fg"
    >
      Sections
    </h2>
    <div class="flex flex-col gap-10">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">FilterPanel</p>
        <div class="flex h-80 overflow-hidden rounded-md border border-rim bg-canvas">
          <div class="min-w-0 flex-1 bg-panel p-4">
            <p class="sf-body-md text-fg-dim">Main content area</p>
          </div>
          <FilterPanel
            vocabulary={filterVocabulary}
            selectedTags={filterTags}
            activeFilterCount={filterTags.size}
            showingCount={14}
            totalCount={23}
            onclose={() => {}}
            ontoggle={(tag, checked) => {
              const next = new Set(filterTags);
              if (checked) next.add(tag);
              else next.delete(tag);
              filterTags = next;
            }}
          />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">NoteDetailPane — note detail Section</p>
        <Button variant="normal" size="small" onclick={() => (showNoteDetail = !showNoteDetail)}>
          {showNoteDetail ? "Hide" : "Show"} note detail
        </Button>
        {#if showNoteDetail}
          <div class="relative mt-4 h-[28rem] overflow-hidden rounded-md border border-rim bg-canvas">
            <NoteDetailPane
              item={demoNoteItem}
              onclose={() => (showNoteDetail = false)}
              onrefresh={() => {}}
              ondelete={() => {}}
            />
          </div>
        {/if}
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-regions">
    <h2
      id="sec-regions"
      class="mb-6 sf-headline-sm text-fg"
    >
      Regions
    </h2>
    <div class="flex flex-col gap-10">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">TitleBar — App Region</p>
        <p class="sf-body-md text-fg-muted mb-3">
          Scribe capture opens via New Note. Dictate is a persistent title-bar action. Note
          editor shows a back button top-left.
        </p>
        <div class="flex flex-col gap-3">
          <div class="overflow-hidden rounded-md border border-rim">
            <TitleBar onNewNote={() => {}} />
          </div>
          <div class="overflow-hidden rounded-md border border-rim">
            <TitleBar onNewNote={() => {}} onBack={() => {}} backLabel="Notes" />
          </div>
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">AppSidebar — App Areas</p>
        <div class="flex h-72 overflow-hidden rounded-md border border-rim bg-canvas">
          <AppSidebar
            activeRoute={demoRoute}
            onnavigate={(route) => {
              demoRoute = route;
            }}
          />
          <div class="min-w-0 flex-1 bg-panel p-4">
            <p class="sf-body-md text-fg-dim">Route: {demoRoute}</p>
          </div>
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">SettingsSidebar — Settings Area</p>
        <div class="flex h-72 overflow-hidden rounded-md border border-rim bg-canvas">
          <SettingsSidebar
            activeTab={demoSettingsTab}
            ontabchange={(tab) => {
              demoSettingsTab = tab;
            }}
            onback={() => {}}
          />
          <div class="min-w-0 flex-1 bg-panel p-4">
            <p class="sf-body-md text-fg-dim">Tab: {demoSettingsTab}</p>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-onboarding">
    <h2
      id="sec-onboarding"
      class="mb-6 sf-headline-sm text-fg"
    >
      Onboarding
    </h2>
    <p class="mb-8 max-w-3xl sf-body-md text-fg-dim">
      Full-height step screens. Shown in bounded frames — some steps call Tauri APIs when
      running inside the app.
    </p>
    <div class="flex flex-col gap-10">
      <div>
        <p class="sf-section-label text-fg-dim mb-3">WelcomeStep</p>
        <div class="h-[28rem] overflow-hidden rounded-md border border-rim bg-panel p-6">
          <WelcomeStep onStart={() => {}} onSkip={() => {}} />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">FeatureTourStep</p>
        <div class="h-[28rem] overflow-hidden rounded-md border border-rim bg-panel p-6">
          <FeatureTourStep onBack={() => {}} onFinish={() => {}} />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">PermissionsStep</p>
        <div class="h-[28rem] overflow-hidden rounded-md border border-rim bg-panel p-6">
          <PermissionsStep onBack={() => {}} onNext={() => {}} />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">ModelDownloadStep</p>
        <div class="h-[28rem] overflow-hidden rounded-md border border-rim bg-panel p-6">
          <ModelDownloadStep onNext={() => {}} />
        </div>
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-3">DictatePracticeStep</p>
        <div class="h-[28rem] overflow-hidden rounded-md border border-rim bg-panel p-6">
          <DictatePracticeStep onBack={() => {}} onNext={() => {}} />
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-prototypes">
    <h2
      id="sec-prototypes"
      class="mb-2 sf-headline-sm text-fg"
    >
      Prototypes
    </h2>
    <p class="mb-2 max-w-3xl sf-body-md text-fg-dim leading-relaxed">
      Exploratory surfaces — not implemented in the app. Query spec:
      <code class="text-brand">ds get prototypes.scribeRecordingBar</code>
    </p>
    <p class="mb-8 max-w-3xl sf-body-md text-fg-muted leading-relaxed">
      Problem: Scribe recording is easy to forget when the main window sits behind
      other apps. The system orange mic dot is generic (any app). macOS does not let
      third-party apps recolor the real menu bar — the prototype uses a
      <span class="sf-body-md-strong text-fg-dim">full-width top band</span> with a
      <span class="sf-body-md-strong text-fg-dim">destructive border accent</span> (not a solid red fill).
    </p>

    <div class="flex flex-col gap-10">
      <!-- Simulated display top -->
      <div>
        <p class="mb-3 sf-section-label text-fg-dim">
          Scribe recording bar (proposal v2) — border accent, one timer
        </p>
        <div
          class="overflow-hidden rounded-md border border-rim bg-canvas shadow-ambient"
          role="img"
          aria-label="Prototype: full-width recording bar with destructive bottom border"
        >
          <div class={scribeBarProtoClass}>
            <button
              type="button"
              class="flex shrink-0 cursor-pointer items-center gap-2 rounded-sm px-2 py-1 transition-colors hover:bg-fill focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-focus"
              aria-label="Open Scribe window (prototype)"
            >
              <span class="sf-label-sm text-fg">Scribe</span>
              <RecordingStatusDot status="recording" />
              <RecordingTimer elapsedSeconds={prototypeElapsedSeconds} />
            </button>
            <div class="min-w-0 flex-1 max-w-56">
              <AudioWaveFormVisualizer
                micLevel={prototypeMicLevel}
                speakerLevel={prototypeSpeakerLevel}
                speakerEnabled={true}
                size="small"
              />
            </div>
            <div class="flex shrink-0 items-center gap-2">
              <Button variant="primary" size="small">Stop and Save</Button>
              <IconButton
                variant="destructive"
                size="small"
                icon={Trash2}
                aria-label="Discard recording (prototype)"
              />
            </div>
          </div>
          <div class="h-20 border-t border-rim/60 bg-canvas px-4 pt-3" aria-hidden="true">
            <p class="sf-label-sm text-fg-muted">
              Your apps sit below — this band is our always-on-top window at the top of
              the screen, not the macOS menu bar.
            </p>
          </div>
        </div>
        <ul class="mt-4 max-w-3xl list-disc space-y-1 pl-5 sf-label-md text-fg-dim">
          <li>
            <code class="text-brand">border-b-2 border-destructive</code> on
            <code class="text-brand">bg-panel</code> — not solid
            <code class="text-brand">bg-destructive</code>
          </li>
          <li>Red dot + timer = recording state; no separate “Recording” label</li>
          <li>Stop and Save + discard always on the bar, even if Dictate is active</li>
          <li>Left cluster is clickable — opens/focuses Scribe (hover <code>bg-fill</code>)</li>
          <li>Always-on-top while <code>RECORDING</code>; never steals focus</li>
        </ul>
      </div>

      <!-- Dictate reference -->
      <div>
        <p class="mb-3 sf-section-label text-fg-dim">
          Dictate HUD (shipped) — corner pill for comparison
        </p>
        <div
          class="relative h-32 overflow-hidden rounded-md border border-rim bg-canvas"
          role="img"
          aria-label="Reference: Dictate recording pill top-right"
        >
          <div class={`absolute right-3 top-3 ${dictateHudClass}`}>
            <div class="flex items-center gap-4">
              <div class="flex items-center gap-2">
                <RecordingStatusDot status="recording" pulseWhileRecording={false} />
                <RecordingTimer elapsedSeconds={prototypeElapsedSeconds} />
              </div>
              <AudioWaveFormVisualizer
                micLevel={prototypeMicLevel}
                speakerLevel={0}
                speakerEnabled={false}
                size="small"
              />
            </div>
            <IconButton variant="normal" size="small" icon={Close} aria-label="Close" />
          </div>
        </div>
      </div>

      <!-- Side by side -->
      <div>
        <p class="mb-3 sf-section-label text-fg-dim">
          Both at once (different jobs, different chrome)
        </p>
        <p class="mb-3 max-w-3xl sf-label-md text-fg-muted">
          Scribe keeps full actions while Dictate runs — two independent sessions. Dictate
          pill matches shipped markup (top-right).
        </p>
        <div
          class="overflow-hidden rounded-md border border-rim bg-canvas"
          role="img"
          aria-label="Prototype: Dictate pill and Scribe bar shown together"
        >
          <div class={scribeBarProtoClass}>
            <button
              type="button"
              class="flex shrink-0 cursor-pointer items-center gap-2 rounded-sm px-2 py-1 transition-colors hover:bg-fill"
              aria-label="Open Scribe window (prototype)"
            >
              <span class="sf-label-sm text-fg">Scribe</span>
              <RecordingStatusDot status="recording" />
              <RecordingTimer elapsedSeconds={prototypeElapsedSeconds} />
            </button>
            <div class="min-w-0 flex-1 max-w-56">
              <AudioWaveFormVisualizer
                micLevel={prototypeMicLevel}
                speakerLevel={prototypeSpeakerLevel}
                speakerEnabled={true}
                size="small"
              />
            </div>
            <div class="flex shrink-0 items-center gap-2">
              <Button variant="primary" size="small">Stop and Save</Button>
              <IconButton
                variant="destructive"
                size="small"
                icon={Trash2}
                aria-label="Discard recording (prototype)"
              />
            </div>
          </div>
          <div class="relative h-24">
            <div class={`absolute right-3 top-3 ${dictateHudClass}`}>
              <div class="flex items-center gap-4">
                <div class="flex items-center gap-2">
                  <RecordingStatusDot status="recording" pulseWhileRecording={false} />
                  <RecordingTimer elapsedSeconds={42} />
                </div>
                <AudioWaveFormVisualizer
                  micLevel={prototypeMicLevel}
                  speakerLevel={0}
                  speakerEnabled={false}
                  size="small"
                />
              </div>
              <IconButton variant="normal" size="small" icon={Close} aria-label="Close" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-audio">
    <h2
      id="sec-audio"
      class="mb-6 sf-headline-sm text-fg"
    >
      Audio (static demo)
    </h2>
    <div class="flex flex-col gap-10 lg:flex-row lg:items-start">
      <div class="flex flex-col items-center gap-8">
        <div class="flex flex-col items-center gap-2">
          <p class="sf-section-label text-fg-dim">Normal (with speaker audio)</p>
          <AudioWaveFormVisualizer
            micLevel={0.55}
            speakerLevel={0.35}
            speakerEnabled={true}
            size="normal"
          />
        </div>
        <div class="flex flex-col items-center gap-2">
          <p class="sf-section-label text-fg-dim">Normal (without speaker audio)</p>
          <AudioWaveFormVisualizer
            micLevel={0.55}
            speakerLevel={0.35}
            speakerEnabled={false}
            size="normal"
          />
        </div>
        <div class="flex flex-col items-center gap-2">
          <p class="sf-section-label text-fg-dim">Dictate capture HUD</p>
          <div class={dictateHudClass}>
            <div class="flex gap-4">
              <div
                class="flex items-center gap-2"
              >
                <RecordingStatusDot status="recording" />
                <RecordingTimer elapsedSeconds={94} />
              </div>
              <AudioWaveFormVisualizer
                micLevel={0.55}
                speakerLevel={0.35}
                speakerEnabled={false}
                size="small"
              />
            </div>
            <IconButton variant="normal" size="small" icon={Close} aria-label="Close" />
          </div>
        </div>
      </div>
      <div class="flex flex-col gap-6">
        <div>
          <p class="sf-section-label text-fg-dim mb-2">
            RecordingStatusDot
          </p>
          <div class="flex flex-wrap gap-4">
            {#each recordingStatuses as s (s)}
              <div class="flex items-center gap-2">
                <RecordingStatusDot status={s} />
                <span class="sf-label-md text-fg-dim">{s}</span>
              </div>
            {/each}
          </div>
        </div>
        <div>
          <p class="sf-section-label text-fg-dim mb-2">RecordingTimer</p>
          <RecordingTimer elapsedSeconds={3723} />
        </div>
        <div>
          <p class="sf-section-label text-fg-dim mb-2">Scribe capture header</p>
          <div class="flex justify-between items-end min-h-11">
            <EditableTitleField bind:value={titleDemo} />
            <div class="flex gap-2 items-center">
              <RecordingTimer elapsedSeconds={3723} />
              <RecordingStatusDot status="recording" />
            </div>
          </div>
        </div>
        <div>
          <p class="sf-section-label text-fg-dim mb-2">
            StackProgressBar indeterminate
          </p>
          <StackProgressBar
            variant="large"
            indeterminate
            sequence={stackProgressSequence}
          />
        </div>
        <div>
          <p class="sf-section-label text-fg-dim mb-2">
            StackProgressBar Large (variant defaults)
          </p>
          <StackProgressBar
            variant="large"
            progress={62}
            sequence={stackProgressSequence}
          />
        </div>
        <div>
          <p class="sf-section-label text-fg-dim mb-2">
            StackProgressBar Small (current state only)
          </p>
          <div class="w-60 pr-2">
          <StackProgressBar
            variant="small"
            progress={62}
            sequence={stackProgressSequence}
          />
          </div>
        </div>

      </div>
    </div>
  </section>

  <section class="mb-20" aria-labelledby="sec-legacy-notes">
    <h2
      id="sec-legacy-notes"
      class="mb-6 sf-headline-sm text-fg"
    >
      Legacy note UI
    </h2>
    <p class="mb-6 max-w-2xl sf-body-md text-fg-dim">
      Chat-style inline note components — deprecated in favour of the unified Note Body
      markdown area. See <code class="text-brand">CONTEXT.md</code> deprecated terms.
    </p>
    <div class="flex max-w-md flex-col gap-6">
      <div>
        <p class="sf-section-label text-fg-dim mb-2">InlineNote (deprecated)</p>
        <InlineNoteCard
          note={{ id: "x", text: "Standalone card.", recordedAtMs: 73_000 }}
          selected={false}
        />
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-2">NotesList (deprecated)</p>
        <NotesList notes={notesDemo} bind:selectedId={selectedNoteId} />
      </div>
      <div>
        <p class="sf-section-label text-fg-dim mb-2">NoteComposer (deprecated)</p>
        <NoteComposer bind:value={draft} onSubmit={onComposerDone} />
      </div>
    </div>
  </section>
</main>

<style>
  /* Subtle recording tint — keeps fg-on-panel contrast, softer than solid destructive */
  .scribe-recording-bar-proto {
    background: color-mix(in srgb, var(--color-destructive) 10%, var(--color-panel));
  }
</style>
