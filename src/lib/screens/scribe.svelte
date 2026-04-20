<script lang="ts">
	import Accordion from "@components/accordion/Accordion.svelte";
	import AccordionItem from "@components/accordion/AccordionItem.svelte";
	import Button from "@components/Button.svelte";
	import CircularAudioVisualizer from "@components/audio/CircularAudioVisualizer.svelte";
	import RecordingStatusDot from "@components/audio/RecordingStatusDot.svelte";
	import RecordingTimer from "@components/audio/RecordingTimer.svelte";
	import Checkbox from "@components/form/Checkbox.svelte";
	import ConfigField from "@components/form/ConfigField.svelte";
	import EditableTitleField from "@components/form/EditableTitleField.svelte";
	import ToggleSwitch from "@components/form/ToggleSwitch.svelte";
	import NoteComposer from "@components/notes/NoteComposer.svelte";
	import NotesList from "@components/notes/NotesList.svelte";
	import Bin from "lucide-svelte/icons/trash-2";
	import type { Note } from "@components/notes/NoteCard.svelte";

	let speakerEnabled = $state(false);
	let selectedMic = $state("macbook-pro-mic");
	let micName = $state("Mic");
	let speakerName = $state("Speaker");
	let modelSmall = $state(true);
	let modelMedium = $state(false);
	let savePath = $state("path/");
	let sendEnabled = $state(false);
	let fileName = $state("File name");
	let noteDraft = $state("");
	let selectedNoteId = $state<string | null>(null);

	const micOptions = [
		{ value: "macbook-pro-mic", label: "MacBook Pro Mic" },
		{ value: "external-usb-mic", label: "External USB Mic" },
	];

	let notes = $state<Note[]>([
		{ id: "n1", text: "notes", createdAt: Date.now() - 400_000 },
		{ id: "n2", text: "notes", createdAt: Date.now() - 120_000 },
	]);

	function addNote() {
		const text = noteDraft.trim();
		if (!text) return;
		notes = [...notes, { id: crypto.randomUUID(), text, createdAt: Date.now() }];
		noteDraft = "";
	}
</script>

<div class="mx-auto flex max-w-5xl flex-col gap-4 text-on-surface">
	<section class="flex h-screen flex-col overflow-hidden bg-surface-container-lowest">
		<header class="flex min-h-14 items-end justify-between px-5 py-2 border-b border-b-surface-container-low">
			<div class="min-w-0 flex-1">
				<EditableTitleField bind:value={fileName} />
			</div>
			<div class="ml-4 flex items-center gap-2">
				<Button variant="normal" iconOnly icon={Bin} class="text-error-container"/>
				<RecordingTimer elapsedSeconds={0} />
				<RecordingStatusDot status="recording" />
			</div>
		</header>

		<div class="grid min-h-0 flex-1 grid-cols-[1.05fr_0.95fr] items-stretch">
			<div class="flex min-h-0 flex-col">
				<div class="px-4 py-5">
					<div class="mx-auto max-w-sm">
						<CircularAudioVisualizer
							micLevel={0.6}
							speakerLevel={0.4}
							speakerEnabled={speakerEnabled}
							innerBaseScale={0.28}
							ampInner={0.12}
							outerScale={1.22}
							ampOuter={0.12}
						/>
					</div>
				</div>

				<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
					<Accordion defaultOpenId="basic">
						<AccordionItem id="basic" title="Basic">
							<div class="space-y-4">
								<ConfigField
									label="Selected mic"
									mode="select"
									options={micOptions}
									bind:value={selectedMic}
								/>
								<div class="space-y-3 rounded-md">
									<div class="flex items-center justify-between">
										<span class="text-label-sm font-semibold tracking-stamped uppercase">Speaker on</span>
										<ToggleSwitch bind:checked={speakerEnabled} aria-label="Toggle speaker layer" />
									</div>
									{#if speakerEnabled}
										<ConfigField label="Mic name" mode="action" bind:value={micName} buttonLabel="Edit" />
										<ConfigField
											label="Speaker name"
											mode="action"
											bind:value={speakerName}
											buttonLabel="Edit"
										/>
									{/if}
								</div>
							</div>
						</AccordionItem>
						<AccordionItem id="advanced" title="Advanced">
							<div class="space-y-4">
								<div class="space-y-2">
									<p class="text-label-sm font-semibold tracking-stamped text-on-surface/80 uppercase">Models</p>
									<div class="flex items-center gap-4">
										<Checkbox bind:checked={modelSmall} label="small" />
										<Checkbox bind:checked={modelMedium} label="medium" />
									</div>
								</div>
								<ConfigField
									label="Save to"
									mode="action"
									bind:value={savePath}
									buttonLabel="Change"
									onButtonClick={() => {}}
								/>
								<div class="flex items-center justify-between">
									<span class="text-label-sm font-semibold tracking-stamped uppercase">Send</span>
									<ToggleSwitch bind:checked={sendEnabled} aria-label="Toggle send option" />
								</div>
							</div>
							<a href="/design-system">to design system</a>
						</AccordionItem>
					</Accordion>
				</div>

				<footer class="flex items-center justify-between px-4 py-3">
					<Button variant="primary">Stop and </Button>
				</footer>
			</div>

			<div class="flex min-h-0 flex-col bg-surface-container-lowest p-3 border-l border-l-surface-container-low">
				<p class="mb-2 font-data text-label-md tracking-stamped uppercase text-on-surface/80">add notes</p>
				<div class="min-h-0 flex-1 overflow-y-auto">
					<div class="h-full rounded-md">
						<NotesList notes={notes} bind:selectedId={selectedNoteId} />
					</div>
				</div>
				<NoteComposer bind:value={noteDraft} onSubmit={addNote} />
			</div>
		</div>
	</section>
</div>
