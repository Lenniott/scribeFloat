<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import Waveform from '@components/indicators/Waveform.svelte';
	import StatusDot from '@primitives/display/StatusDot.svelte';
	import RecordingTimer from '@primitives/display/RecordingTimer.svelte';
	import IconButton from '@components/controls/IconButton.svelte';
	import ToggleSwitch from '@components/controls/Toggle.svelte';
	import FieldRow from '@primitives/form/FieldRow.svelte';
	import { Trash2, Settings2 } from 'lucide-svelte';

	type StripPhase = 'idle' | 'recording' | 'transcribing';

	type ScribePayload = {
		state: string;
		error?: string;
	};

	let {
		noteId,
		ontranscriptready,
		recordingActive = $bindable(false),
	}: {
		noteId: string;
		ontranscriptready?: () => void;
		recordingActive?: boolean;
	} = $props();

	let phase = $state<StripPhase>('idle');
	let audioLevel = $state(0);
	let speakerLevel = $state(0);
	let elapsedMs = $state(0);
	let settingsOpen = $state(false);
	let selectedMic = $state('');
	let captureSpeaker = $state(false);
	let includeTimestamps = $state(true);
	let micOptions = $state([{ value: '', label: 'System Default' }]);
	let errorMessage = $state('');
	let awaitingAttach = false;
	let settingsAnchor: HTMLDivElement | undefined = $state();
	let timerInterval: ReturnType<typeof setInterval> | null = null;
	const speakerEnabledForWaveform = $derived(captureSpeaker);
	const elapsedSeconds = $derived(Math.floor(elapsedMs / 1000));
	let unlisteners: UnlistenFn[] = [];

	$effect(() => {
		recordingActive = phase === 'recording';
	});

	function startElapsedTimer() {
		stopElapsedTimer();
		const start = Date.now();
		elapsedMs = 0;
		timerInterval = setInterval(() => {
			elapsedMs = Date.now() - start;
		}, 100);
	}

	function stopElapsedTimer() {
		if (timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	}

	async function loadSettings() {
		includeTimestamps = await invoke<boolean>('scribe_get_include_timestamps').catch(() => true);
		const devices = await invoke<string[]>('scribe_list_input_devices').catch(() => []);
		micOptions = [
			{ value: '', label: 'System Default' },
			...devices.map((d) => ({ value: d, label: d })),
		];
		const [preferredMic] = await invoke<[string | null, string | null]>(
			'settings_get_preferred_audio_devices',
		).catch(() => [null, null] as [string | null, string | null]);
		selectedMic = preferredMic ?? '';
		captureSpeaker = await invoke<boolean>('settings_get_scribe_capture_speaker').catch(() => false);
	}

	async function startRecording() {
		errorMessage = '';
		await invoke('scribe_set_attach_note', { noteId });
		await invoke('scribe_start', {
			preferredMic: selectedMic || null,
			preferredSpeaker: null,
			captureSpeaker,
		});
		phase = 'recording';
		startElapsedTimer();
	}

	async function stopAndSave() {
		if (phase !== 'recording') return;
		phase = 'transcribing';
		stopElapsedTimer();
		audioLevel = 0;
		speakerLevel = 0;
		awaitingAttach = true;
		try {
			await invoke('scribe_stop_and_save', { title: null });
		} catch (e) {
			awaitingAttach = false;
			phase = 'idle';
			errorMessage = String(e);
		}
	}

	async function discardRecording() {
		stopElapsedTimer();
		audioLevel = 0;
		speakerLevel = 0;
		awaitingAttach = false;
		await invoke('scribe_set_attach_note', { noteId: null }).catch(() => {});
		await invoke('scribe_cancel').catch(() => {});
		phase = 'idle';
	}

	async function handleDone() {
		if (!awaitingAttach) return;
		awaitingAttach = false;
		try {
			await invoke('note_attach_transcript', { id: noteId });
			phase = 'idle';
			ontranscriptready?.();
		} catch (e) {
			phase = 'idle';
			errorMessage = String(e);
		}
	}

	function handleScribeEvent(p: ScribePayload) {
		switch (p.state) {
			case 'IDLE':
				if (!awaitingAttach) {
					phase = 'idle';
					stopElapsedTimer();
					audioLevel = 0;
					speakerLevel = 0;
				}
				break;
			case 'RECORDING':
				phase = 'recording';
				if (!timerInterval) startElapsedTimer();
				break;
			case 'TRANSCRIBING':
				phase = 'transcribing';
				stopElapsedTimer();
				audioLevel = 0;
				speakerLevel = 0;
				break;
			case 'DONE':
				void handleDone();
				break;
			case 'ERROR':
				awaitingAttach = false;
				phase = 'idle';
				stopElapsedTimer();
				errorMessage = p.error ?? 'Recording failed';
				break;
		}
	}

	function closeSettings() {
		settingsOpen = false;
	}

	function handleWindowClick(e: MouseEvent) {
		if (!settingsOpen) return;
		const target = e.target;
		if (target instanceof Node && settingsAnchor?.contains(target)) return;
		closeSettings();
	}

	onMount(async () => {
		await loadSettings();
		unlisteners.push(
			await listen<ScribePayload>('scribe://state-changed', (e) =>
				handleScribeEvent(e.payload),
			),
		);
		unlisteners.push(
			await listen<number>('scribe://audio-level', (e) => {
				audioLevel = e.payload;
			}),
		);
		unlisteners.push(
			await listen<number>('scribe://speaker-level', (e) => {
				speakerLevel = e.payload;
			}),
		);
	});

	onDestroy(() => {
		stopElapsedTimer();
		for (const ul of unlisteners) ul();
	});
</script>

<svelte:window onclick={handleWindowClick} />

{#if phase === 'idle'}
	<div class="flex min-h-10 shrink-0 items-center gap-3 border-b border-card px-4">
		<button type="button" class="sf-label-md text-fg hover:text-brand" onclick={startRecording}>
			Start Recording
		</button>
		<div class="relative ml-auto" bind:this={settingsAnchor}>
			<IconButton
				aria-label="Recording settings"
				icon={Settings2}
				size="small"
				variant="normal"
				onclick={(e) => {
					e.stopPropagation();
					settingsOpen = !settingsOpen;
				}}
			/>
			{#if settingsOpen}
				<div
					class="absolute right-0 top-full z-50 mt-1 w-64 rounded-md border border-fill bg-card p-3 shadow-lg space-y-4"
					role="group"
					aria-label="Recording settings"
				>
					<FieldRow
						label="Microphone"
						id="recording-strip-mic"
						mode="select"
						options={micOptions}
						bind:value={selectedMic}
					/>
					<ToggleSwitch
						label="Speaker capture"
						checked={captureSpeaker}
						onchange={(v) => {
							captureSpeaker = v;
							void invoke('scribe_toggle_speaker_capture', { enabled: v });
						}}
					/>
					<ToggleSwitch
						label="Timestamps"
						checked={includeTimestamps}
						onchange={(v) => {
							includeTimestamps = v;
							void invoke('scribe_set_include_timestamps', { enabled: v });
						}}
					/>
				</div>
			{/if}
		</div>
		{#if errorMessage}
			<p class="sf-body-md text-destructive">{errorMessage}</p>
		{/if}
	</div>
{:else if phase === 'recording'}
	<div class="flex min-h-14 shrink-0 items-center gap-3 border-b border-card px-4">
		<Waveform micLevel={audioLevel} speakerLevel={speakerLevel} speakerEnabled={speakerEnabledForWaveform} size="small" />
		<StatusDot status="recording" />
		<RecordingTimer {elapsedSeconds} />
		<button type="button" class="sf-label-md text-fg hover:text-brand" onclick={stopAndSave}>
			Stop &amp; Save
		</button>
		<IconButton
			aria-label="Discard recording"
			icon={Trash2}
			size="small"
			variant="normal"
			onclick={discardRecording}
		/>
	</div>
{:else}
	<div class="flex min-h-10 shrink-0 items-center gap-3 border-b border-card px-4">
		<p class="sf-body-md text-fg-dim">Transcribing…</p>
	</div>
{/if}
