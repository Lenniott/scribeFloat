<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { openPath } from '@tauri-apps/plugin-opener';

	import Accordion from '@components/accordion/Accordion.svelte';
	import AccordionItem from '@components/accordion/AccordionItem.svelte';
	import Button from '@components/Button.svelte';
	import IconButton from '@components/IconButton.svelte';
	import CircularAudioVisualizer from '@components/audio/CircularAudioVisualizer.svelte';
	import RecordingStatusDot from '@components/audio/RecordingStatusDot.svelte';
	import RecordingTimer from '@components/audio/RecordingTimer.svelte';
	import ConfigField from '@components/form/ConfigField.svelte';
	import EditableTitleField from '@components/form/EditableTitleField.svelte';
	import ToggleSwitch from '@components/form/ToggleSwitch.svelte';
	import NoteComposer from '@components/notes/NoteComposer.svelte';
	import NotesList from '@components/notes/NotesList.svelte';
	import Bin from 'lucide-svelte/icons/trash-2';
	import type { Note } from '@components/notes/NoteCard.svelte';

	// ── State machine ─────────────────────────────────────────────────────────
	type Phase = 'idle' | 'recording' | 'transcribing' | 'done' | 'no_model' | 'error';
	let phase = $state<Phase>('idle');
	let errorMessage = $state('');
	let transcriptPath = $state('');

	// ── Model download ─────────────────────────────────────────────────────────
	let modelReady = $state(true);
	let downloadProgress = $state(0);

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
	let speakerEnabled = $state(false);
	let selectedMic = $state('');
	let micName = $state('Mic');
	let speakerName = $state('Speaker');
	let noteDraft = $state('');
	let notes = $state<Note[]>([]);
	let selectedNoteId = $state<string | null>(null);

	const micOptions = [{ value: '', label: 'System Default' }];

	// ── Backend events ────────────────────────────────────────────────────────
	type ScribePayload = {
		state: string;
		progress?: number;
		transcript_path?: string;
		wav_path?: string;
		error?: string;
	};

	function handleScribeEvent(p: ScribePayload) {
		switch (p.state) {
			case 'RECORDING':
				phase = 'recording';
				startTimer();
				break;
			case 'TRANSCRIBING':
				phase = 'transcribing';
				stopTimer();
				break;
			case 'DONE':
				phase = 'done';
				transcriptPath = p.transcript_path ?? '';
				break;
			case 'NO_MODEL':
				phase = 'no_model';
				stopTimer();
				break;
			case 'ERROR':
				phase = 'error';
				errorMessage = p.error ?? 'Unknown error';
				stopTimer();
				break;
		}
	}

	// ── Actions ───────────────────────────────────────────────────────────────
	async function startRecording() {
		try {
			await invoke('scribe_start');
		} catch (e) {
			phase = 'error';
			errorMessage = String(e);
		}
	}

	async function stopAndSave() {
		try {
			await invoke('scribe_stop_and_save', { title: fileName || 'Recording' });
		} catch (e) {
			phase = 'error';
			errorMessage = String(e);
			stopTimer();
		}
	}

	async function cancel() {
		stopTimer();
		notes = [];
		elapsedSeconds = 0;
		try {
			await invoke('scribe_cancel');
		} catch (_) {}
		phase = 'idle';
	}

	async function recordAgain() {
		notes = [];
		elapsedSeconds = 0;
		transcriptPath = '';
		errorMessage = '';
		await startRecording();
	}

	function addNote() {
		const text = noteDraft.trim();
		if (!text) return;
		notes = [...notes, { id: crypto.randomUUID(), text, createdAt: Date.now() }];
		noteDraft = '';
		invoke('scribe_add_note', { text }).catch(() => {});
	}

	async function openTranscript() {
		if (transcriptPath) await openPath(transcriptPath);
	}

	// ── Lifecycle ─────────────────────────────────────────────────────────────
	let unlisteners: UnlistenFn[] = [];

	onMount(async () => {
		modelReady = await invoke<boolean>('model_status').catch(() => false);

		const ul1 = await listen<ScribePayload>('scribe://state-changed', (e) =>
			handleScribeEvent(e.payload)
		);
		const ul2 = await listen<{ progress: number }>('model://download-progress', (e) => {
			downloadProgress = e.payload.progress;
			if (e.payload.progress >= 1.0) modelReady = true;
		});
		unlisteners = [ul1, ul2];

		await startRecording();
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
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
				{#if !modelReady}
					<span class="font-data text-label-sm text-on-surface/60 uppercase tracking-stamped">
						Model {Math.round(downloadProgress * 100)}%
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
			<div class="flex min-h-0 flex-col">
				<div class="mx-auto mb-4 max-w-48">
					<CircularAudioVisualizer
						micLevel={phase === 'recording' ? 0.6 : 0}
						speakerLevel={phase === 'recording' && speakerEnabled ? 0.4 : 0}
						{speakerEnabled}
						innerBaseScale={0.28}
						ampInner={0.12}
						outerScale={1.22}
						ampOuter={0.12}
					/>
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
										<span class="text-label-sm font-semibold tracking-stamped uppercase"
											>Speaker on</span
										>
										<ToggleSwitch bind:checked={speakerEnabled} aria-label="Toggle speaker" />
									</div>
									{#if speakerEnabled}
										<ConfigField
											label="Mic name"
											mode="action"
											bind:value={micName}
											buttonLabel="Edit"
										/>
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

					{:else if phase === 'transcribing'}
						<span class="font-data text-label-sm text-on-surface/60 uppercase tracking-stamped">
							Transcribing…
						</span>

					{:else if phase === 'done'}
						<div class="flex min-w-0 flex-1 flex-col gap-2">
							<p class="font-data text-label-sm text-on-surface/60 uppercase tracking-stamped">
								Transcript saved
							</p>
							<p class="truncate text-body-sm text-on-surface/80" title={transcriptPath}>
								{transcriptPath}
							</p>
							<div class="flex gap-2">
								<Button variant="primary" onclick={openTranscript}>Open</Button>
								<Button variant="secondary" onclick={recordAgain}>Record Again</Button>
							</div>
						</div>

					{:else if phase === 'no_model'}
						<div class="flex flex-col gap-2">
							<p class="text-label-sm text-on-surface/80">
								{#if modelReady}
									Model ready — audio was saved but not transcribed.
								{:else}
									Model downloading ({Math.round(downloadProgress * 100)}%) — audio was saved.
								{/if}
							</p>
							<Button variant="secondary" onclick={recordAgain}>Try Again</Button>
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
