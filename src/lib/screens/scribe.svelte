<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	import Accordion from '@components/accordion/Accordion.svelte';
	import AccordionItem from '@components/accordion/AccordionItem.svelte';
	import Button from '@components/Button.svelte';
	import IconButton from '@components/IconButton.svelte';
	import Modal from '@components/Modal.svelte';
	import RecordingStatusDot from '@components/audio/RecordingStatusDot.svelte';
	import RecordingTimer from '@components/audio/RecordingTimer.svelte';
	import AudioWaveFormVisualizer from '@lib/components/audio/AudioWaveFormVisualizer.svelte';
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

type Props = {
	processingStart?: (title: string) => void;
	autoStart?: boolean;
};

let { processingStart, autoStart = true }: Props = $props();

	// ── State machine ─────────────────────────────────────────────────────────
	type Phase = 'idle' | 'recording' | 'no_model' | 'error';
	let phase = $state<Phase>('idle');
	let errorMessage = $state('');

	// ── Model download ─────────────────────────────────────────────────────────
	const modelStore = createModelDownloadStore();
	let modelUnlisteners: (() => void)[] = [];
	let modelSetupOpen = $state(false);
	let settingsOpen = $state(false);
	let discardConfirmOpen = $state(false);
	let discardInProgress = $state(false);
	let startInProgress = false;

	const modelReady = $derived(modelStore.models.some((m) => m.downloaded));
	const canCloseModelSetup = $derived(modelStore.models.some((m) => m.selected && m.downloaded));
	const downloadedModelOptions = $derived(
		modelStore.models
			.filter((m) => m.downloaded)
			.map((m) => ({ value: m.id, label: m.label })),
	);

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
	function defaultTitle() {
		const now = new Date();
		const pad = (n: number) => n.toString().padStart(2, '0');
		return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}-${pad(now.getMinutes())}`;
	}

	let fileName = $state(defaultTitle());
	let selectedMic = $state('');
	let selectedModelId = $state('');
	let noteDraft = $state('');
	let notes = $state<Note[]>([]);
	let selectedNoteId = $state<string | null>(null);
	let includeTimestamps = $state(true);
	let micLevel = $state(0);
	let micOptions = $state([{ value: '', label: 'System Default' }]);

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
		if (startInProgress || phase === 'recording') return;
		startInProgress = true;
		try {
			await invoke('scribe_start', { preferredMic: selectedMic || null });
		} catch (e) {
			phase = 'error';
			errorMessage = String(e);
		} finally {
			startInProgress = false;
		}
	}

	async function maybeAutoStartRecording() {
		if (!autoStart || !modelReady || modelSetupOpen || discardConfirmOpen || discardInProgress) {
			return;
		}
		if (phase === 'idle') {
			await startRecording();
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

	async function discardRecording() {
		discardInProgress = true;
		await cancel();
		discardConfirmOpen = false;
		discardInProgress = false;
		await getCurrentWindow().close();
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
		await modelStore.refresh();
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
	let unlistenFocus: (() => void) | undefined;

	onMount(async () => {
		includeTimestamps = await invoke<boolean>('scribe_get_include_timestamps').catch(() => true);
		modelUnlisteners = await modelStore.subscribe();
		await modelStore.refresh();

		// Populate mic list
		const devices = await invoke<string[]>('scribe_list_input_devices').catch(() => []);
		micOptions = [
			{ value: '', label: 'System Default' },
			...devices.map((d) => ({ value: d, label: d })),
		];

		// Sync model selector with the currently selected model
		const sel = modelStore.models.find((m) => m.selected && m.downloaded);
		if (sel) selectedModelId = sel.id;

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
		unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
			if (focused) void maybeAutoStartRecording();
		});

		await maybeAutoStartRecording();
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
		unlistenFocus?.();
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
						aria-label="Discard recording"
						onclick={() => (discardConfirmOpen = true)}
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
								<div class="flex flex-col gap-1.5 text-left">
									<label for="mic-select" class="font-data text-label-sm font-normal tracking-widest text-on-surface/80 uppercase">
										Selected mic
									</label>
									<select
										id="mic-select"
										bind:value={selectedMic}
										onchange={async () => {
											if (phase === 'recording') {
												stopTimer();
												notes = [];
												elapsedSeconds = 0;
												micLevel = 0;
												try { await invoke('scribe_cancel'); } catch (_) {}
												phase = 'idle';
												await startRecording();
											}
										}}
										class="h-8 rounded-md border-0 border-b border-transparent bg-surface-container-lowest py-2 pr-8 pl-2 text-body-md text-on-surface focus:ring-active focus:bg-surface-container-high focus:ring-0 focus:outline-none"
									>
										{#each micOptions as opt (opt.value)}
											<option value={opt.value}>{opt.label}</option>
										{/each}
									</select>
								</div>
								{#if downloadedModelOptions.length > 0}
									<div class="flex flex-col gap-1.5 text-left">
										<label for="model-select" class="font-data text-label-sm font-normal tracking-widest text-on-surface/80 uppercase">
											Model
										</label>
										<select
											id="model-select"
											value={selectedModelId}
											onchange={async (e) => {
												const id = (e.currentTarget as HTMLSelectElement).value;
												selectedModelId = id;
												await modelStore.select(id);
											}}
											class="h-8 rounded-md border-0 border-b border-transparent bg-surface-container-lowest py-2 pr-8 pl-2 text-body-md text-on-surface focus:ring-active focus:bg-surface-container-high focus:ring-0 focus:outline-none"
										>
											{#each downloadedModelOptions as opt (opt.value)}
												<option value={opt.value}>{opt.label}</option>
											{/each}
										</select>
									</div>
								{/if}
								<div class="flex items-center justify-between">
									<span class="font-data text-label-sm font-normal tracking-stamped uppercase">
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
				<footer class="flex items-center gap-3 py-3">
					{#if phase === 'idle'}
						{#if autoStart}
							<span class="font-data text-label-sm text-on-surface/50 uppercase tracking-stamped">
								Starting…
							</span>
						{:else}
							<Button variant="primary" onclick={startRecording}>Start Recording</Button>
						{/if}

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
	onClose={closeModelSetup}
/>

<Modal
	open={discardConfirmOpen}
	title="Discard recording?"
	description="Are you sure you want to discard this recording? This cannot be undone."
	maxWidthClass="max-w-md"
	closeDisabled={discardInProgress}
	onClose={() => (discardConfirmOpen = false)}
>
	{#snippet footer()}
		<div class="flex gap-2">
			<Button
				variant="secondary"
				disabled={discardInProgress}
				onclick={() => (discardConfirmOpen = false)}
			>
				Cancel
			</Button>
			<Button
				variant="destructive"
				disabled={discardInProgress}
				onclick={discardRecording}
			>
				Discard
			</Button>
		</div>
	{/snippet}
</Modal>

{#if settingsOpen}
	<SettingsScreen onClose={() => (settingsOpen = false)} />
{/if}
