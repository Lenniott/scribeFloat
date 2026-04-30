<script lang="ts">
  import Accordion from "@components/accordion/Accordion.svelte";
  import AccordionItem from "@components/accordion/AccordionItem.svelte";
  import SettingsSection from "@components/accordion/SettingsSection.svelte";
  import AudioLayerLegend from "@components/audio/AudioLayerLegend.svelte";
  import RecordingStatusDot from "@components/audio/RecordingStatusDot.svelte";
  import RecordingTimer from "@components/audio/RecordingTimer.svelte";
  import Button from "@components/Button.svelte";
  import IconButton from "@components/IconButton.svelte";
  import Checkbox from "@components/form/Checkbox.svelte";
  import ConfigField from "@components/form/ConfigField.svelte";
  import EditableTitleField from "@components/form/EditableTitleField.svelte";
  import LabeledTextField from "@components/form/LabeledTextField.svelte";
  import OptionGroup from "@components/form/OptionGroup.svelte";
  import StackProgressBar from "@components/form/StackProgressBar.svelte";
  import ToggleSwitch from "@components/form/ToggleSwitch.svelte";
  import TabPage, { type TabPageItem } from "@components/layout/TabPage.svelte";
  import NoteCard from "@components/notes/NoteCard.svelte";
  import NoteComposer from "@components/notes/NoteComposer.svelte";
  import NotesList from "@components/notes/NotesList.svelte";
  import TimestampLabel from "@components/notes/TimestampLabel.svelte";
  import type { Note } from "@components/notes/NoteCard.svelte";
  import { applyThemeMode, type ThemeMode } from "$lib/theme";
  import { X as Close } from "lucide-svelte";
  import ChevronRight from "lucide-svelte/icons/chevron-right";
  import Plus from "lucide-svelte/icons/plus";
  import Trash2 from "lucide-svelte/icons/trash-2";
  import AudioWaveFormVisualizer from "@lib/components/audio/AudioWaveFormVisualizer.svelte";

  /** Local playground state — only for exercising controls, not a real screen */
  let toggleA = $state(false);
  let checkboxA = $state(true);
  let selectValue = $state("a");
  let textA = $state("");
  let titleDemo = $state("Editable title");
  let pathDemo = $state("~/example/path");
  let hotkeyDemo = $state("Cmd+Shift+H");
  let optionDemo = $state("one");
  let notesDemo = $state<Note[]>([
    { id: "n1", text: "Example note body.", recordedAtMs: 120_000 },
  ]);
  let selectedNoteId = $state<string | null>(null);
  let draft = $state("");
  let activePanelTab = $state<string>("setup");
  let activeSectionTab = $state<string>("timers");
  let previewTheme = $state<ThemeMode>("system");

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

  const recordingStatuses = ["idle", "recording", "paused", "error"] as const;

  const colorTokens: { token: string; class: string }[] = [
    { token: "void", class: "bg-void" },
    { token: "surface", class: "bg-surface" },
    { token: "surface-container", class: "bg-surface-container" },
    { token: "primary", class: "bg-primary" },
    { token: "secondary", class: "bg-secondary" },
    { token: "active", class: "bg-active" },
    { token: "surface-lowest", class: "bg-surface-lowest" },
    { token: "surface-low", class: "bg-surface-low" },
    { token: "surface-high", class: "bg-surface-high" },
    {
      token: "surface-highest",
      class: "bg-surface-highest",
    },
    { token: "error", class: "bg-error" },
  ];

  const variants = [
    "primary",
    "secondary",
    "destructive",
    "transparent",
    "normal",
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
  const panelTabs: TabPageItem[] = [
    { id: "setup", label: "Setup" },
    { id: "status", label: "Status" },
    { id: "notes", label: "Notes" },
  ];
  const sectionTabs: TabPageItem[] = [
    { id: "timers", label: "Timers" },
    { id: "recording", label: "Recording" },
  ];

  const themeOptions = [
    { value: "system", label: "System" },
    { value: "dark", label: "Dark" },
    { value: "light", label: "Light" },
  ];

  $effect(() => {
    applyThemeMode(previewTheme);
  });
</script>

<main class="mx-auto text-left p-4">
  <a href="/">scribe</a>
  <header class="mb-14 max-w-2xl">
    <p class="font-mono text-label-sm tracking-stamped text-on-surface/50 uppercase">
      liscribe · design system
    </p>
    <h1 class="text-display-lg font-light tracking-heading text-on-surface">
      Design <span class="font-medium">system</span>
    </h1>
    <p class="mt-3 text-body-md text-on-surface/65 leading-relaxed">
      Geist typography, semantic theme tokens, and the existing component variants:
      <code class="text-primary">primary</code>,
      <code class="text-primary">secondary</code>,
      <code class="text-primary">normal</code>,
      <code class="text-primary">transparent</code>, and
      <code class="text-primary">active</code>.
    </p>
  </header>

  <section class="mb-16" aria-labelledby="sec-theme">
    <h2
      id="sec-theme"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Theme Modes
    </h2>
    <div class="max-w-md rounded-md bg-surface-low p-6">
      <OptionGroup
        name="theme-preview"
        label="Preview theme"
        options={themeOptions}
        bind:selected={previewTheme}
      />
      <p class="mt-4 text-body-md text-on-surface/65">
        The app stores <code class="text-primary">system</code>,
        <code class="text-primary">dark</code>, or
        <code class="text-primary">light</code> in settings and resolves those to document-level
        theme tokens.
      </p>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-colors">
    <h2
      id="sec-colors"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Color Roles
    </h2>
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
      {#each colorTokens as { token, class: c } (token)}
        <div class="flex flex-col gap-2">
          <div class="rounded-md bg-surface-low p-2">
            <div class="h-12 rounded-md {c}"></div>
          </div>
          <span class="font-mono text-label-sm font-normal tracking-stamped text-on-surface/90"
            >{token}</span
          >
        </div>
      {/each}
    </div>
    <p class="text-label-sm mt-4 text-on-surface/50">
      Text: <span class="text-on-surface">on-surface</span> ·
      <span class="text-on-error">on-error</span>
    </p>
  </section>

  <section class="mb-16" aria-labelledby="sec-type">
    <h2
      id="sec-type"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Typography
    </h2>
    <div class="flex flex-col gap-6 bg-surface-low p-6 rounded-md">
      <div>
        <p class="text-label-sm text-on-surface/45 mb-1">
          display-lg · Geist
        </p>
        <p class="text-display-lg font-light tracking-heading text-on-surface">
          Record <span class="font-medium">clearly</span>
        </p>
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-1">
          headline-lg · Geist
        </p>
        <p
          class="text-headline-lg font-light tracking-heading text-on-surface"
        >
          Section header
        </p>
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-1">
          mono label · Geist Mono
        </p>
        <p class="font-mono text-label-sm font-normal tracking-stamped text-on-surface/80 uppercase">
          transcript · input
        </p>
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-1">body-md · Geist</p>
        <p class="text-body-md text-on-surface/90">
          Standard UI copy defaults to light weight with relaxed leading for dense layouts.
        </p>
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-1">label-sm / label-md</p>
        <p class="font-mono text-label-sm font-normal tracking-stamped text-on-surface/80 uppercase">
          Metadata
        </p>
        <p class="text-label-md font-normal text-on-surface/70">
          Secondary label
        </p>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-geo">
    <h2
      id="sec-geo"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Geometry
    </h2>
    <div class="flex flex-wrap items-end gap-6">
      <div class="flex flex-col gap-2">
        <span class="text-label-sm text-on-surface/50">radius-md (4px)</span>
        <div class="h-16 w-16 rounded-md bg-surface-highest"></div>
      </div>
      <div class="flex flex-col gap-2">
        <span class="text-label-sm text-on-surface/50">radius-sm (2px)</span>
        <div class="h-16 w-16 rounded-sm bg-surface-highest"></div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-buttons">
    <h2
      id="sec-buttons"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Button
    </h2>
    <div class="flex flex-col gap-8">
      {#each sizes as size (size)}
        <div>
          <p class="text-label-sm text-on-surface/45 mb-3 uppercase">
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
        <p class="text-label-sm text-on-surface/45 mb-3 uppercase">With icon</p>
        <div class="flex flex-wrap gap-3">
          <Button variant="primary" icon={ChevronRight}>Next</Button>
          <Button variant="secondary" size="small" icon={Plus}>Add</Button>
          <Button variant="destructive" icon={Trash2}>Remove</Button>
        </div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-icon-buttons">
    <h2
      id="sec-icon-buttons"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      IconButton
    </h2>
    <p class="text-body-md text-on-surface/65 mb-6 max-w-xl">
      Icon-only control. Requires <code class="text-primary">aria-label</code>.
      Variants:
      <code class="text-primary">primary</code>,
      <code class="text-primary">destructive</code>,
      <code class="text-primary">normal</code>.
    </p>
    <div class="flex flex-col gap-8">
      {#each sizes as size (size)}
        <div>
          <p class="text-label-sm text-on-surface/45 mb-3 uppercase">
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
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Form
    </h2>
    <div
      class="flex max-w-md flex-col gap-6 bg-surface-low p-6 rounded-md"
    >
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
        <span
          class="font-mono text-label-sm font-normal tracking-stamped text-on-surface/80 uppercase"
          >ToggleSwitch</span
        >
        <ToggleSwitch aria-label="Demo toggle" bind:checked={toggleA} />
      </div>
      <div class="flex items-center justify-between gap-4">
        <span
          class="font-mono text-label-sm font-normal tracking-stamped text-on-surface/80 uppercase"
          >Checkbox</span
        >
        <Checkbox aria-label="Demo checkbox" bind:checked={checkboxA} />
      </div>
      <OptionGroup
        name="ds-option"
        label="OptionGroup"
        options={[
          { value: "one", label: "One" },
          { value: "two", label: "Two" },
        ]}
        bind:selected={optionDemo}
      />
      <ConfigField
        label="ConfigField (path)"
        mode="action"
        bind:value={pathDemo}
        buttonLabel="Change"
        onButtonClick={() => {}}
      />
      <ConfigField
        label="ConfigField (hotkey)"
        mode="action"
        bind:value={hotkeyDemo}
        buttonLabel="Capture"
        onButtonClick={() => {}}
      />
      <div>
        <p class="text-label-sm text-on-surface/45 mb-2">EditableTitleField</p>
        <EditableTitleField bind:value={titleDemo} />
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-tabs">
    <h2
      id="sec-tabs"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      TabPage
    </h2>
    <div class="max-w-2xl">
      <TabPage tabs={panelTabs} bind:activeId={activePanelTab}>
        {#snippet children(activeTab)}
          {#if activeTab?.id === "setup"}
            <p class="text-body-md text-on-surface/80">
              Use this mode when a tab owns the full panel body.
            </p>
          {:else if activeTab?.id === "status"}
            <div class="flex items-center gap-3">
              <RecordingStatusDot status="recording" />
              <RecordingTimer elapsedSeconds={245} />
            </div>
          {:else}
            <NoteComposer bind:value={draft} onSubmit={onComposerDone} />
          {/if}
        {/snippet}
      </TabPage>

      <div class="mt-4">
        <TabPage
          tabs={sectionTabs}
          mode="section"
          bind:activeId={activeSectionTab}
        >
          {#snippet children(activeTab)}
            {#if activeTab?.id === "timers"}
              <div class="flex items-center gap-4">
                <RecordingTimer elapsedSeconds={94} />
                <RecordingTimer elapsedSeconds={3723} />
              </div>
            {:else}
              <div class="flex items-center gap-2">
                <RecordingStatusDot status="idle" />
                <RecordingStatusDot status="recording" />
                <RecordingStatusDot status="paused" />
              </div>
            {/if}
          {/snippet}
        </TabPage>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-acc">
    <h2
      id="sec-acc"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Accordion
    </h2>
    <div class="max-w-md overflow-hidden">
      <Accordion>
        <AccordionItem id="ds-1" title="First section">
          <SettingsSection title="Inner title">
            <p class="text-body-md text-on-surface/75">
              SettingsSection + AccordionItem body.
            </p>
          </SettingsSection>
        </AccordionItem>
        <AccordionItem id="ds-2" title="Second section">
          <p class="text-body-md text-on-surface/75">Another panel.</p>
        </AccordionItem>
      </Accordion>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-audio">
    <h2
      id="sec-audio"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Audio (static demo)
    </h2>
    <div class="flex flex-col gap-10 lg:flex-row lg:items-start">
      <div class="flex flex-col items-center gap-8">
        <div class="flex flex-col items-center gap-2">
          <p class="text-label-sm text-on-surface/45">Normal (with speaker audio)</p>
          <AudioWaveFormVisualizer
            micLevel={0.55}
            speakerLevel={0.35}
            speakerEnabled={true}
            size="normal"
          />
        </div>
        <div class="flex flex-col items-center gap-2">
          <p class="text-label-sm text-on-surface/45">Normal (without speaker audio)</p>
          <AudioWaveFormVisualizer
            micLevel={0.55}
            speakerLevel={0.35}
            speakerEnabled={false}
            size="normal"
          />
        </div>
        <div class="flex flex-col items-center gap-2">
          <p class="text-label-sm text-on-surface/45">DicateRecordScreen</p>
          <div class="flex gap-2 justify-between items-center w-60 py-2 pl-3 pr-2 bg-surface-lowest">
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
          <p class="text-label-sm text-on-surface/45 mb-2">
            RecordingStatusDot
          </p>
          <div class="flex flex-wrap gap-4">
            {#each recordingStatuses as s (s)}
              <div class="flex items-center gap-2">
                <RecordingStatusDot status={s} />
                <span class="text-label-md text-on-surface/70">{s}</span>
              </div>
            {/each}
          </div>
        </div>
        <div>
          <p class="text-label-sm text-on-surface/45 mb-2">RecordingTimer</p>
          <RecordingTimer elapsedSeconds={3723} />
        </div>
        <div>
          <p class="text-label-sm text-on-surface/45 mb-2">ScribeHeader</p>
          <div class="flex justify-between items-end min-h-11">
            <EditableTitleField bind:value={titleDemo} />
            <div class="flex gap-2 items-center">
              <RecordingTimer elapsedSeconds={3723} />
              <RecordingStatusDot status="recording" />
            </div>
          </div>
        </div>
        <div>
          <p class="text-label-sm text-on-surface/45 mb-2">
            StackProgressBar Large (variant defaults)
          </p>
          <StackProgressBar
            variant="large"
            progress={62}
            sequence={stackProgressSequence}
          />
        </div>
        <div>
          <p class="text-label-sm text-on-surface/45 mb-2">
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
        <div>
          <p class="text-label-sm text-on-surface/45 mb-2">AudioLayerLegend</p>
          <div class="rounded-md bg-surface-low px-4 py-3">
            <AudioLayerLegend speakerEnabled={true} />
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="mb-20" aria-labelledby="sec-notes">
    <h2
      id="sec-notes"
      class="mb-6 text-headline-lg font-light tracking-heading text-on-surface"
    >
      Notes
    </h2>
    <div class="flex max-w-md flex-col gap-6">
      <div>
        <p class="text-label-sm text-on-surface/45 mb-2">TimestampLabel</p>
        <TimestampLabel at={94_000} />
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-2">NoteCard</p>
        <NoteCard
          note={{ id: "x", text: "Standalone card.", recordedAtMs: 73_000 }}
          selected={false}
        />
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-2">NotesList</p>
        <NotesList notes={notesDemo} bind:selectedId={selectedNoteId} />
      </div>
      <div>
        <p class="text-label-sm text-on-surface/45 mb-2">NoteComposer</p>
        <NoteComposer bind:value={draft} onSubmit={onComposerDone} />
      </div>
    </div>
  </section>
</main>
