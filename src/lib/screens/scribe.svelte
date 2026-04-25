<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';

	import Accordion from '@components/accordion/Accordion.svelte';
	import AccordionItem from '@components/accordion/AccordionItem.svelte';
	import Button from '@components/Button.svelte';
	import IconButton from '@components/IconButton.svelte';
	import RecordingStatusDot from '@components/audio/RecordingStatusDot.svelte';
	import RecordingTimer from '@components/audio/RecordingTimer.svelte';
	import AudioWaveFormVisualizer from '@lib/components/audio/AudioWaveFormVisualizer.svelte';
	import ConfigField from '@components/form/ConfigField.svelte';
	import EditableTitleField from '@components/form/EditableTitleField.svelte';
	import ToggleSwitch from '@components/form/ToggleSwitch.svelte';
	import ModelSetupModal from '@components/model/ModelSetupModal.svelte';
	import NoteComposer from '@components/notes/NoteComposer.svelte';
	import NotesList from '@components/notes/NotesList.svelte';
	import SettingsScreen from '@lib/screens/settings.svelte';
	import { createModelDownloadStore } from '$lib/stores/modelDownload.svelte';
	import Bin from 'lucide-svelte/icons/trash-2';
	import Cog from 'lucide-svelte/icons/settings-2';
	import type { Note } from '@components/notes/NoteCard.svelte';

	let {
		processingStart,
	}: {
		processingStart?: (title: string) => void;
	} = $props();

	// ── State machine ─────────────────────────────────────────────────────────
	type Phase = 'idle' | 'recording' | 'no_model' | 'error';
	let phase = $state<Phase>('idle');
	let errorMessage = $state('');

	// ── Model download ─────────────────────────────────────────────────────────
	const modelStore = createModelDownloadStore();
	let modelUnlisteners: (() => void)[] = [];
	let modelSetupOpen = $state(false);
	let settingsOpen = $state(false);

	const modelReady = $derived(modelStore.models.some((m) => m.downloaded));
	const canCloseModelSetup = $derived(modelStore.models.some((m) => m.selected && m.downloaded));

	// ── Recording ─────────────────────────────────────────────────────────────
	let elapsedSeconds = $state(0);
	let timerInterval: ReturnType<typeof setInterval> | null = null;

	function startTimer() {
		const start = Date.now();
		elapsedSeconds = 0;
		timerInterval = setInterval(() => {
			elapsedSeconds = Math.floor((Date.now() - start) / 1000);
		}, 1000);
	}

	function stopTimer() {
		if (timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	}

	// ── Session metadata ──────────────────────────────────────────────────────
	let fileName = $state('Recording');
	let selectedMic = $state('');
	let noteDraft = $state('');
	let notes = $state<Note[]>([]);
	let selectedNoteId = $state<string | null>(null);
	let includeTimestamps = $state(true);
	let micLevel = $state(0);

	const micOptions = [{ value: '', label: 'System Default' }];

	// ── Backend events ────────────────────────────────────────────────────────
	type ScribePayload = {
		state: string;
		progress?: number;
		transcript_path?: string;
		wav_path?: string;
		error?: string;
	};

	type BackendNote = {
		id: string;
		text: string;
		recorded_at_ms: number;
	};

	function handleScribeEvent(p: ScribePayload) {
		switch (p.state) {
			case 'RECORDING':
				phase = 'recording';
				startTimer();
				break;
			case 'TRANSCRIBING':
				stopTimer();
				micLevel = 0;
				break;
			case 'DONE':
				stopTimer();
				micLevel = 0;
				break;
			case 'NO_MODEL':
				phase = 'no_model';
				stopTimer();
				micLevel = 0;
				modelSetupOpen = true;
				break;
			case 'ERROR':
				phase = 'error';
				errorMessage = p.error ?? 'Unknown error';
				stopTimer();
				micLevel = 0;
				break;
		}
	}

	// ── Actions ───────────────────────────────────────────────────────────────
	async function startRecording() {
		try {
			await invoke('scribe_start', { preferredMic: selectedMic || null });
		} catch (e) {
			phase = 'error';
			errorMessage = String(e);
		}
	}

	async function stopAndSave() {
		stopTimer();
		micLevel = 0;
		processingStart?.(fileName || 'Recording');
	}

	async function cancel() {
		stopTimer();
		notes = [];
		elapsedSeconds = 0;
		micLevel = 0;
		try {
			await invoke('scribe_cancel');
		} catch (_) {}
		phase = 'idle';
	}

	async function recordAgain() {
		notes = [];
		elapsedSeconds = 0;
		errorMessage = '';
		micLevel = 0;
		await startRecording();
	}

	async function closeModelSetup() {
		modelSetupOpen = false;
		if (canCloseModelSetup && phase !== 'recording') {
			await startRecording();
		}
	}

	async function addNote() {
		const text = noteDraft.trim();
		if (!text) return;
		const draft = noteDraft;
		noteDraft = '';
		const created = await invoke<BackendNote>('scribe_add_note', { text: draft }).catch(() => null);
		if (!created) return;
		notes = [
			...notes,
			{ id: created.id, text: created.text, recordedAtMs: created.recorded_at_ms },
		];
	}

	// ── Lifecycle ─────────────────────────────────────────────────────────────
	let unlisteners: UnlistenFn[] = [];

	onMount(async () => {
		includeTimestamps = await invoke<boolean>('scribe_get_include_timestamps').catch(() => true);
		modelUnlisteners = await modelStore.subscribe();
		await modelStore.refresh();

		if (!modelReady) {
			modelSetupOpen = true;
		}

		const ul1 = await listen<ScribePayload>('scribe://state-changed', (e) =>
			handleScribeEvent(e.payload),
		);
		const ul2 = await listen<number>('scribe://audio-level', (e) => {
			micLevel = e.payload ?? 0;
		});
		unlisteners = [ul1, ul2];

		if (modelReady) {
			await startRecording();
		}
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
		modelUnlisteners.forEach((u) => u());
		stopTimer();
	});
</script>

<div class="mx-auto flex max-w-5xl flex-col gap-4 text-on-surface">
	<section class="flex h-screen flex-col overflow-hidden bg-surface-container-lowest">

		<!-- Header -->
		<header class="flex min-h-14 items-end justify-between border-b border-b-surface-container-low px-5 py-2">
			<div class="min-w-0 flex-1">
				<EditableTitleField bind:value={fileName} />
			</div>
			<div class="ml-4 flex items-center gap-2">
				<IconButton
					variant="normal"
					size="small"
					icon={Cog}
					aria-label="Open settings"
					onclick={() => (settingsOpen = true)}
				/>
				{#if modelStore.activeDownloadModelId}
					<span class="font-data text-label-sm text-on-surface/60 uppercase tracking-stamped">
						Model {Math.round((modelStore.progressByModel[modelStore.activeDownloadModelId] ?? 0) * 100)}%
					</span>
				{/if}
				{#if phase === 'recording'}
					<IconButton
						variant="destructive"
						size="small"
						icon={Bin}
						aria-label="Cancel recording"
						onclick={cancel}
					/>
					<RecordingTimer {elapsedSeconds} />
					<RecordingStatusDot status="recording" />
				{/if}
			</div>
		</header>

		<!-- Body -->
		<div class="grid min-h-0 flex-1 grid-cols-[1.05fr_0.95fr] items-stretch">

			<!-- Left: visualizer + settings -->
			<div class="flex min-h-0 flex-col px-4 py-3">
				<AudioWaveFormVisualizer
					micLevel={phase === 'recording' ? micLevel : 0}
					speakerLevel={0}
					speakerEnabled={false}
					size="normal"
				/>

				<div class="min-h-0 flex-1 overflow-y-auto">
					<Accordion defaultOpenId="basic">
						<AccordionItem id="basic" title="Basic">
							<div class="space-y-4">
								<ConfigField
									label="Selected mic"
									mode="select"
									options={micOptions}
									bind:value={selectedMic}
								/>
								<div class="flex items-center justify-between">
									<span class="text-label-sm font-semibold tracking-stamped uppercase">
										Transcript timestamps
									</span>
									<ToggleSwitch
										checked={includeTimestamps}
										aria-label="Toggle transcript timestamps"
										onchange={async (next) => {
											const prev = includeTimestamps;
											includeTimestamps = next;
											await invoke('scribe_set_include_timestamps', { enabled: next }).catch(() => {
												includeTimestamps = prev;
											});
										}}
									/>
								</div>
							</div>
						</AccordionItem>
					</Accordion>
				</div>

				<!-- Footer -->
				<footer class="flex items-center gap-3 px-4 py-3">
					{#if phase === 'idle'}
						<span class="font-data text-label-sm text-on-surface/50 uppercase tracking-stamped">
							Starting…
						</span>

					{:else if phase === 'recording'}
						<Button variant="primary" onclick={stopAndSave}>Stop and Save</Button>

					{:else if phase === 'no_model'}
						<div class="flex flex-col gap-2">
							<p class="text-label-sm text-on-surface/80">
								No model selected. Open model settings to download and select a model.
							</p>
							<Button variant="secondary" onclick={() => (modelSetupOpen = true)}>Open model settings</Button>
						</div>

					{:else if phase === 'error'}
						<div class="flex flex-col gap-2">
							<p class="text-label-sm text-error">{errorMessage}</p>
							<Button variant="secondary" onclick={recordAgain}>Try Again</Button>
						</div>
					{/if}
				</footer>
			</div>

			<!-- Right: notes -->
			<div
				class="flex min-h-0 flex-col border-l border-l-surface-container-low bg-surface-container-lowest p-3"
			>
				<p class="mb-2 font-data text-label-md tracking-stamped text-on-surface/80 uppercase">
					add notes
				</p>
				<div class="min-h-0 flex-1 overflow-y-auto">
					<div class="h-full rounded-md">
						<NotesList {notes} bind:selectedId={selectedNoteId} />
					</div>
				</div>
				{#if phase === 'recording'}
					<NoteComposer bind:value={noteDraft} onSubmit={addNote} />
				{/if}
			</div>
		</div>
	</section>
</div>

<ModelSetupModal
	open={modelSetupOpen}
	models={modelStore.models}
	progressByModel={modelStore.progressByModel}
	downloadingByModel={modelStore.downloadingByModel}
	statusByModel={modelStore.statusByModel}
	errorMessage={modelStore.error}
	canClose={true}
	onDownload={modelStore.download}
	onSelect={modelStore.select}
	onClose={closeModelSetup}
/>

{#if settingsOpen}
	<SettingsScreen onClose={() => (settingsOpen = false)} />
{/if}
