<script lang="ts">
	import Accordion from "@components/accordion/Accordion.svelte";
	import AccordionItem from "@components/accordion/AccordionItem.svelte";
	import SettingsSection from "@components/accordion/SettingsSection.svelte";
	import AudioLayerLegend from "@components/audio/AudioLayerLegend.svelte";
	import CircularAudioVisualizer from "@components/audio/CircularAudioVisualizer.svelte";
	import RecordingStatusDot from "@components/audio/RecordingStatusDot.svelte";
	import RecordingTimer from "@components/audio/RecordingTimer.svelte";
	import Button from "@components/Button.svelte";
	import Checkbox from "@components/form/Checkbox.svelte";
	import ConfigField from "@components/form/ConfigField.svelte";
	import EditableTitleField from "@components/form/EditableTitleField.svelte";
	import LabeledTextField from "@components/form/LabeledTextField.svelte";
	import OptionGroup from "@components/form/OptionGroup.svelte";
	import ProgressBar from "@components/form/ProgressBar.svelte";
	import ToggleSwitch from "@components/form/ToggleSwitch.svelte";
	import TabPage, { type TabPageItem } from "@components/layout/TabPage.svelte";
	import NoteCard from "@components/notes/NoteCard.svelte";
	import NoteComposer from "@components/notes/NoteComposer.svelte";
	import NotesList from "@components/notes/NotesList.svelte";
	import TimestampLabel from "@components/notes/TimestampLabel.svelte";
	import type { Note } from "@components/notes/NoteCard.svelte";
	import { X as Close } from "lucide-svelte";
	import ChevronRight from "lucide-svelte/icons/chevron-right";
	import Plus from "lucide-svelte/icons/plus";
	import Trash2 from "lucide-svelte/icons/trash-2";

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
		{ id: "n1", text: "Example note body.", createdAt: Date.now() - 120_000 },
	]);
	let selectedNoteId = $state<string | null>(null);
	let draft = $state("");
	let activePanelTab = $state<string>("setup");
	let activeSectionTab = $state<string>("timers");

	const selectOptions = [
		{ value: "a", label: "Option A" },
		{ value: "b", label: "Option B" },
	];

	function appendDemoNote(text: string) {
		notesDemo = [...notesDemo, { id: crypto.randomUUID(), text, createdAt: Date.now() }];
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
		{ token: "primary", class: "bg-primary" },
		{ token: "primary-container", class: "bg-primary-container" },
		{ token: "surface-container-lowest", class: "bg-surface-container-lowest" },
		{ token: "surface-container-low", class: "bg-surface-container-low" },
		{ token: "surface-container-high", class: "bg-surface-container-high" },
		{ token: "surface-container-highest", class: "bg-surface-container-highest" },
		{ token: "surface-variant", class: "bg-surface-variant" },
		{ token: "tertiary", class: "bg-tertiary" },
		{ token: "error-container", class: "bg-error-container" },
	];

	const variants = ["primary", "secondary", "destructive", "tertiary", "normal"] as const;
	const sizes = ["normal", "small"] as const;
	const progressSequence = [
		{ label: "model small", complete: true },
		{ label: "File created", complete: true },
		{ label: "Model medium", complete: true },
		{ label: "File created", complete: false },
		{ label: "Model large", complete: false },
		{ label: "Result export", complete: false },
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
</script>

<main class="mx-auto text-left p-4">
    <a href="/">scribe</a>
	<header class="mb-14 max-w-2xl">
		<p class="text-label-sm tracking-stamped text-on-surface/50 uppercase">Liscribe · tokens</p>
		<h1 class="font-data text-display-lg text-on-surface">Design system</h1>
		<p class="mt-3 text-body-md text-on-surface/65 leading-relaxed">
			Surfaces, type, and components from <code class="text-primary">context/DESIGN.md</code>. This page is
			for reviewing primitives only — not a product layout.
		</p>
	</header>

	<section class="mb-16" aria-labelledby="sec-colors">
		<h2 id="sec-colors" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Color
		</h2>
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4">
			{#each colorTokens as { token, class: c } (token)}
				<div class="flex flex-col gap-2">
					<div class="rounded-md bg-surface-container-low p-2">
						<div class="h-12 rounded-md {c}"></div>
					</div>
					<span class="text-label-sm font-semibold text-on-surface/90">{token}</span>
				</div>
			{/each}
		</div>
		<p class="text-label-sm mt-4 text-on-surface/50">
			Text: <span class="text-on-surface">on-surface</span> ·
			<span class="text-on-primary-container">on-primary-container</span> ·
			<span class="text-on-error-container">on-error-container</span>
		</p>
	</section>

	<section class="mb-16" aria-labelledby="sec-type">
		<h2 id="sec-type" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Typography
		</h2>
		<div class="flex flex-col gap-6 bg-surface-container-low p-6 rounded-md">
			<div>
				<p class="text-label-sm text-on-surface/45 mb-1">display-lg · Space Grotesk</p>
				<p class="font-data text-display-lg text-on-surface">00:00</p>
			</div>
			<div>
				<p class="text-label-sm text-on-surface/45 mb-1">headline-sm · stamped</p>
				<p class="font-data text-headline-sm tracking-stamped text-on-surface uppercase">Section header</p>
			</div>
			<div>
				<p class="text-label-sm text-on-surface/45 mb-1">body-md · Inter</p>
				<p class="text-body-md text-on-surface/90">
					Standard UI copy. Line height tuned for dense technical layouts.
				</p>
			</div>
			<div>
				<p class="text-label-sm text-on-surface/45 mb-1">label-sm / label-md</p>
				<p class="text-label-sm font-semibold text-on-surface/80 uppercase">Metadata</p>
				<p class="text-label-md font-medium text-on-surface/70">Secondary label</p>
			</div>
		</div>
	</section>

	<section class="mb-16" aria-labelledby="sec-geo">
		<h2 id="sec-geo" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Geometry & depth
		</h2>
		<div class="flex flex-wrap items-end gap-6">
			<div class="flex flex-col gap-2">
				<span class="text-label-sm text-on-surface/50">radius-md (4px)</span>
				<div class="h-16 w-16 rounded-md bg-surface-container-highest"></div>
			</div>
			<div class="flex flex-col gap-2">
				<span class="text-label-sm text-on-surface/50">radius-sm (2px)</span>
				<div class="h-16 w-16 rounded-sm bg-surface-container-highest"></div>
			</div>
			<div class="flex flex-col gap-2">
				<span class="text-label-sm text-on-surface/50">shadow-ambient</span>
				<div class="h-16 w-24 rounded-md bg-surface-container-high shadow-ambient"></div>
			</div>
		</div>
	</section>

	<section class="mb-16" aria-labelledby="sec-buttons">
		<h2 id="sec-buttons" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Button
		</h2>
		<div class="flex flex-col gap-8">
			{#each sizes as size (size)}
				<div>
					<p class="text-label-sm text-on-surface/45 mb-3 uppercase">Size · {size}</p>
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
			<div>
				<p class="text-label-sm text-on-surface/45 mb-3 uppercase">Icon only</p>
				<div class="flex flex-wrap gap-3">
					<Button variant="primary" icon={Plus} iconOnly aria-label="Add" />
					<Button variant="normal" size="small" icon={ChevronRight} iconOnly aria-label="More" />
				</div>
			</div>
		</div>
	</section>

	<section class="mb-16" aria-labelledby="sec-forms">
		<h2 id="sec-forms" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Form
		</h2>
		<div class="flex max-w-md flex-col gap-6 bg-surface-container-low p-6 rounded-md">
			<ConfigField label="ConfigField (select)" mode="select" options={selectOptions} bind:value={selectValue} />
			<LabeledTextField label="LabeledTextField" bind:value={textA} placeholder="Placeholder" />
			<div class="flex items-center justify-between gap-4">
				<span class="text-label-sm font-semibold tracking-wide text-on-surface/80 uppercase">ToggleSwitch</span>
				<ToggleSwitch aria-label="Demo toggle" bind:checked={toggleA} />
			</div>
			<div class="flex items-center justify-between gap-4">
				<span class="text-label-sm font-semibold tracking-wide text-on-surface/80 uppercase">Checkbox</span>
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
		<h2 id="sec-tabs" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			TabPage
		</h2>
		<div class="max-w-2xl">
			<TabPage tabs={panelTabs} bind:activeId={activePanelTab}>
				{#snippet children(activeTab)}
					{#if activeTab?.id === "setup"}
						<p class="text-body-md text-on-surface/80">Use this mode when a tab owns the full panel body.</p>
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
				<TabPage tabs={sectionTabs} mode="section" bind:activeId={activeSectionTab}>
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
		<h2 id="sec-acc" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Accordion
		</h2>
		<div class="max-w-md bg-surface-container-lowest rounded-md overflow-hidden">
			<Accordion>
				<AccordionItem id="ds-1" title="First section">
					<SettingsSection title="Inner title">
						<p class="text-body-md text-on-surface/75">SettingsSection + AccordionItem body.</p>
					</SettingsSection>
				</AccordionItem>
				<AccordionItem id="ds-2" title="Second section">
					<p class="text-body-md text-on-surface/75">Another panel.</p>
				</AccordionItem>
			</Accordion>
		</div>
	</section>

	<section class="mb-16" aria-labelledby="sec-audio">
		<h2 id="sec-audio" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Audio (static demo)
		</h2>
		<p class="text-label-sm text-on-surface/50 mb-6 max-w-xl">
			Fixed levels. The dual waveform is <code class="text-primary">DualRadialWaveform</code> inside
			<code class="text-primary">CircularAudioVisualizer</code> — not a separate product screen.
		</p>
		<div class="flex flex-col gap-10 lg:flex-row lg:items-start">
			<div class="flex flex-col items-center gap-8">
				<div class="flex flex-col items-center gap-2">
					<p class="text-label-sm text-on-surface/45">Scribe Dual CircularAudioVisualizer</p>
					<CircularAudioVisualizer
						micLevel={0.55}
						speakerLevel={0.35}
						innerBaseScale={0.28}
						ampInner={0.13}
						outerScale={1.25}
						ampOuter={0.15}
						speakerEnabled={true}
					/>
				</div>

				<div class="flex flex-col items-center gap-2">
				<p class="text-label-sm text-on-surface/45">Compact with center indicator</p>
				<CircularAudioVisualizer
					size={120}
					micLevel={0.55}
					speakerLevel={0.35}
					innerBaseScale={0.25}
					ampInner={0.12}
					outerScale={1.2}
					ampOuter={0.13}
					speakerEnabled={false}
					showLegend={false}
				>
					{#snippet children()}
						<div class="flex items-center gap-1.5">
							<RecordingStatusDot status="recording" />
							<RecordingTimer elapsedSeconds={94} />
						</div>
					{/snippet}
				</CircularAudioVisualizer>
				</div>
				<div class="flex flex-col items-center gap-2">
					<p class="text-label-sm text-on-surface/45">DicateRecordScreen</p>
					<div class="flex gap-2 items-center">
						<CircularAudioVisualizer
							size={120}
							micLevel={0.55}
							speakerLevel={0.35}
							innerBaseScale={0.25}
							ampInner={0.12}
							outerScale={1.2}
							ampOuter={0.13}
							speakerEnabled={false}
							showLegend={false}
						>
							{#snippet children()}
								<div class="flex items-center gap-1.5">
									<RecordingStatusDot status="recording" />
									<RecordingTimer elapsedSeconds={94} />
								</div>
							{/snippet}
						</CircularAudioVisualizer>
						<div class="flex w-14 justify-end">
						<Button variant="normal" iconOnly icon={Close}/>
						</div>
					</div>
				</div>
			</div>
			<div class="flex flex-col gap-6">
				<div>
					<p class="text-label-sm text-on-surface/45 mb-2">RecordingStatusDot</p>
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
					<p class="text-label-sm text-on-surface/45 mb-2">ProgressBarScribe (windowed sequence)</p>
					<ProgressBar progress={40} sequence={progressSequence} sequenceMode="window" uiSize="lg" />
				</div>
				<div>
					<p class="text-label-sm text-on-surface/45 mb-2">ProgressBarDictate (current stage)</p>
					<ProgressBar progress={100} sequence={progressSequence} sequenceMode="current" uiSize="sm" />
				</div>
				<div>
					<p class="text-label-sm text-on-surface/45 mb-2">AudioLayerLegend</p>
					<div class="rounded-md bg-surface-container-low px-4 py-3">
						<AudioLayerLegend speakerEnabled={true} />
					</div>
				</div>
			</div>
		</div>
	</section>

	<section class="mb-20" aria-labelledby="sec-notes">
		<h2 id="sec-notes" class="font-data text-headline-sm mb-6 tracking-stamped text-on-surface/80 uppercase">
			Notes
		</h2>
		<div class="flex max-w-md flex-col gap-6">
			<div>
				<p class="text-label-sm text-on-surface/45 mb-2">TimestampLabel</p>
				<TimestampLabel at={Date.now()} />
			</div>
			<div>
				<p class="text-label-sm text-on-surface/45 mb-2">NoteCard</p>
				<NoteCard
					note={{ id: "x", text: "Standalone card.", createdAt: Date.now() }}
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