<script lang="ts">
  import Accordion from "@lib/components/patterns/Accordion.svelte";
  import AccordionItem from "@lib/components/ui/nav/AccordionRow.svelte";
  import SettingsSection from "@lib/components/primitives/form/SettingsSection.svelte";
  import RecordingStatusDot from "@lib/components/primitives/display/StatusDot.svelte";
  import RecordingTimer from "@lib/components/primitives/display/RecordingTimer.svelte";
  import Button from "@lib/components/ui/controls/Button.svelte";
  import IconButton from "@lib/components/ui/controls/IconButton.svelte";
  import Checkbox from "@lib/components/primitives/form/Checkbox.svelte";
  import ConfigField from "@lib/components/primitives/form/FieldRow.svelte";
  import EditableTitleField from "@lib/components/ui/controls/EditableTitle.svelte";
  import LabeledTextField from "@lib/components/primitives/form/TextField.svelte";
  import OptionGroup from "@lib/components/ui/controls/OptionGroup.svelte";
  import StackProgressBar from "@lib/components/primitives/display/ProgressBar.svelte";
  import ToggleSwitch from "@lib/components/ui/controls/Toggle.svelte";
  import NoteCard from "@lib/components/ui/cards/InlineNote.svelte";
  import NoteComposer from "@lib/components/patterns/NoteComposer.svelte";
  import NotesList from "@lib/components/patterns/NoteList.svelte";
  import TimestampLabel from "@lib/components/primitives/display/Timestamp.svelte";
  import type { Note } from "@lib/components/ui/cards/InlineNote.svelte";
  import { applyThemeMode, type ThemeMode } from "$lib/theme";
  import { X as Close } from "lucide-svelte";
  import ChevronRight from "lucide-svelte/icons/chevron-right";
  import Plus from "lucide-svelte/icons/plus";
  import Trash2 from "lucide-svelte/icons/trash-2";
  import AudioWaveFormVisualizer from "@lib/components/ui/indicators/Waveform.svelte";

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

<main class="mx-auto min-h-screen bg-canvas text-left p-4">
  <a href="/">scribe</a>
  <header class="mb-14 max-w-2xl">
    <p class="font-mono text-label-sm tracking-stamped text-fg/50 uppercase">
      ScribeFloat · design system
    </p>
    <h1 class="text-display-lg font-light tracking-heading text-fg">
      Design <span class="font-medium">system</span>
    </h1>
    <p class="mt-3 text-body-md text-fg/65 leading-relaxed">
      Geist typography, semantic theme tokens, and the existing component variants:
      <code class="text-brand">primary</code>,
      <code class="text-brand">normal</code>,
      <code class="text-brand">ghost</code>,
      <code class="text-brand">destructive</code>, and
      <code class="text-brand">active</code>.
    </p>
  </header>

  <section class="mb-16" aria-labelledby="sec-theme">
    <h2
      id="sec-theme"
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
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
      <p class="mt-4 text-body-md text-fg/65">
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
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Color Roles
    </h2>
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
      {#each colorTokens as { token, class: c } (token)}
        <div class="flex flex-col gap-2">
          <div class="rounded-md bg-card p-2">
            <div class="h-12 rounded-md {c}"></div>
          </div>
          <span class="font-mono text-label-sm font-normal tracking-stamped text-fg/90"
            >{token}</span
          >
        </div>
      {/each}
    </div>
    <p class="text-label-sm mt-4 text-fg/50">
      Utilities mirror tokens — use <code class="text-fg/80">bg-*</code>,
      <code class="text-fg/80">text-*</code>, or
      <code class="text-fg/80">border-*</code> with the same name (for example
      <code class="text-fg/80">border-active</code>).
    </p>
  </section>

  <section class="mb-16" aria-labelledby="sec-type">
    <h2
      id="sec-type"
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Typography
    </h2>
    <div class="flex flex-col gap-6 bg-card p-6 rounded-md">
      <div>
        <p class="text-label-sm text-fg/45 mb-1">
          display-lg · Geist
        </p>
        <p class="text-display-lg font-light tracking-heading text-fg">
          Record <span class="font-medium">clearly</span>
        </p>
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-1">
          headline-lg · Geist
        </p>
        <p
          class="text-headline-lg font-light tracking-heading text-fg"
        >
          Section header
        </p>
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-1">
          mono label · Geist Mono
        </p>
        <p class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
          transcript · input
        </p>
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-1">body-md · Geist</p>
        <p class="text-body-md text-fg/90">
          Standard UI copy defaults to light weight with relaxed leading for dense layouts.
        </p>
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-1">label-sm / label-md</p>
        <p class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
          Metadata
        </p>
        <p class="text-label-md font-normal text-fg/70">
          Secondary label
        </p>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-geo">
    <h2
      id="sec-geo"
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Geometry
    </h2>
    <div class="flex flex-wrap items-end gap-6">
      <div class="flex flex-col gap-2">
        <span class="text-label-sm text-fg/50">radius-md (4px)</span>
        <div class="h-16 w-16 rounded-md bg-card"></div>
      </div>
      <div class="flex flex-col gap-2">
        <span class="text-label-sm text-fg/50">radius-sm (2px)</span>
        <div class="h-16 w-16 rounded-sm bg-card"></div>
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-buttons">
    <h2
      id="sec-buttons"
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Button
    </h2>
    <div class="flex flex-col gap-8">
      {#each sizes as size (size)}
        <div>
          <p class="text-label-sm text-fg/45 mb-3 uppercase">
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
        <p class="text-label-sm text-fg/45 mb-3 uppercase">With icon</p>
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
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      IconButton
    </h2>
    <p class="text-body-md text-fg/65 mb-6 max-w-xl">
      Icon-only control. Requires <code class="text-brand">aria-label</code>.
      Variants:
      <code class="text-brand">primary</code>,
      <code class="text-brand">destructive</code>,
      <code class="text-brand">normal</code>.
    </p>
    <div class="flex flex-col gap-8">
      {#each sizes as size (size)}
        <div>
          <p class="text-label-sm text-fg/45 mb-3 uppercase">
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
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Form
    </h2>
    <div
      class="flex max-w-md flex-col gap-6 bg-card p-6 rounded-md"
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
        <ToggleSwitch label="Demo toggle" bind:checked={toggleA} />
      </div>
      <div class="flex items-center justify-between gap-4">
        <span
          class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase"
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
        <p class="text-label-sm text-fg/45 mb-2">EditableTitleField</p>
        <EditableTitleField bind:value={titleDemo} />
      </div>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-acc">
    <h2
      id="sec-acc"
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Accordion
    </h2>
    <div class="max-w-md">
      <Accordion>
        <AccordionItem id="ds-1" title="First section">
          <SettingsSection title="Inner title">
            <p class="text-body-md text-fg/75">
              SettingsSection + AccordionItem body.
            </p>
          </SettingsSection>
        </AccordionItem>
        <AccordionItem id="ds-2" title="Second section">
          <p class="text-body-md text-fg/75">Another panel.</p>
        </AccordionItem>
      </Accordion>
    </div>
  </section>

  <section class="mb-16" aria-labelledby="sec-prototypes">
    <h2
      id="sec-prototypes"
      class="mb-2 text-headline-lg font-light tracking-heading text-fg"
    >
      Prototypes
    </h2>
    <p class="mb-2 max-w-3xl text-body-md text-fg/65 leading-relaxed">
      Exploratory surfaces — not implemented in the app. Query spec:
      <code class="text-brand">ds get prototypes.scribeRecordingBar</code>
    </p>
    <p class="mb-8 max-w-3xl text-body-md text-fg/55 leading-relaxed">
      Problem: Scribe recording is easy to forget when the main window sits behind
      other apps. The system orange mic dot is generic (any app). macOS does not let
      third-party apps recolor the real menu bar — the prototype uses a
      <span class="font-medium text-fg/75">full-width top band</span> with a
      <span class="font-medium text-fg/75">destructive border accent</span> (not a solid red fill).
    </p>

    <div class="flex flex-col gap-10">
      <!-- Simulated display top -->
      <div>
        <p class="mb-3 font-mono text-label-sm tracking-stamped text-fg/45 uppercase">
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
              <span class="font-mono text-label-sm tracking-stamped uppercase">Scribe</span>
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
            <p class="text-label-sm text-fg-muted">
              Your apps sit below — this band is our always-on-top window at the top of
              the screen, not the macOS menu bar.
            </p>
          </div>
        </div>
        <ul class="mt-4 max-w-3xl list-disc space-y-1 pl-5 text-label-md text-fg/60">
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
        <p class="mb-3 font-mono text-label-sm tracking-stamped text-fg/45 uppercase">
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
        <p class="mb-3 font-mono text-label-sm tracking-stamped text-fg/45 uppercase">
          Both at once (different jobs, different chrome)
        </p>
        <p class="mb-3 max-w-3xl text-label-md text-fg/55">
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
              <span class="font-mono text-label-sm tracking-stamped uppercase">Scribe</span>
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
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Audio (static demo)
    </h2>
    <div class="flex flex-col gap-10 lg:flex-row lg:items-start">
      <div class="flex flex-col items-center gap-8">
        <div class="flex flex-col items-center gap-2">
          <p class="text-label-sm text-fg/45">Normal (with speaker audio)</p>
          <AudioWaveFormVisualizer
            micLevel={0.55}
            speakerLevel={0.35}
            speakerEnabled={true}
            size="normal"
          />
        </div>
        <div class="flex flex-col items-center gap-2">
          <p class="text-label-sm text-fg/45">Normal (without speaker audio)</p>
          <AudioWaveFormVisualizer
            micLevel={0.55}
            speakerLevel={0.35}
            speakerEnabled={false}
            size="normal"
          />
        </div>
        <div class="flex flex-col items-center gap-2">
          <p class="text-label-sm text-fg/45">DicateRecordScreen</p>
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
          <p class="text-label-sm text-fg/45 mb-2">
            RecordingStatusDot
          </p>
          <div class="flex flex-wrap gap-4">
            {#each recordingStatuses as s (s)}
              <div class="flex items-center gap-2">
                <RecordingStatusDot status={s} />
                <span class="text-label-md text-fg/70">{s}</span>
              </div>
            {/each}
          </div>
        </div>
        <div>
          <p class="text-label-sm text-fg/45 mb-2">RecordingTimer</p>
          <RecordingTimer elapsedSeconds={3723} />
        </div>
        <div>
          <p class="text-label-sm text-fg/45 mb-2">ScribeHeader</p>
          <div class="flex justify-between items-end min-h-11">
            <EditableTitleField bind:value={titleDemo} />
            <div class="flex gap-2 items-center">
              <RecordingTimer elapsedSeconds={3723} />
              <RecordingStatusDot status="recording" />
            </div>
          </div>
        </div>
        <div>
          <p class="text-label-sm text-fg/45 mb-2">
            StackProgressBar Large (variant defaults)
          </p>
          <StackProgressBar
            variant="large"
            progress={62}
            sequence={stackProgressSequence}
          />
        </div>
        <div>
          <p class="text-label-sm text-fg/45 mb-2">
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

  <section class="mb-20" aria-labelledby="sec-notes">
    <h2
      id="sec-notes"
      class="mb-6 text-headline-lg font-light tracking-heading text-fg"
    >
      Notes
    </h2>
    <div class="flex max-w-md flex-col gap-6">
      <div>
        <p class="text-label-sm text-fg/45 mb-2">TimestampLabel</p>
        <TimestampLabel at={94_000} />
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-2">NoteCard</p>
        <NoteCard
          note={{ id: "x", text: "Standalone card.", recordedAtMs: 73_000 }}
          selected={false}
        />
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-2">NotesList</p>
        <NotesList notes={notesDemo} bind:selectedId={selectedNoteId} />
      </div>
      <div>
        <p class="text-label-sm text-fg/45 mb-2">NoteComposer</p>
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
