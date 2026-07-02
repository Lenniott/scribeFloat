<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { onDestroy, onMount } from 'svelte';
	import { CircleCheckBig, Mic } from 'lucide-svelte';
	import Button from '@components/controls/Button.svelte';
	import TextField from '@primitives/form/TextField.svelte';
	import StepShell from '@primitives/layout/StepFrame.svelte';
	import { appErrorMessage } from '@utils/types';
	import { resolveSelectedMic } from '@utils/micOptions';

	type ClipStatus = {
		clip_id: string;
		duration_s: number;
		speech_s: number;
		purity: number;
		state: 'pending' | 'recording' | 'safe' | 'optimal' | 'failed';
	};

	type ClipResult = {
		duration_s: number;
		speech_s: number;
		purity: number;
		accepted: boolean;
	};

	type ModelStatus = {
		downloaded: boolean;
		path: string;
	};

	let {
		onBack,
		onNext,
		isFirstTime = true,
		prefilledName = 'You',
		lockedName = false,
	}: {
		onBack: () => void;
		onNext: () => void;
		isFirstTime?: boolean;
		prefilledName?: string;
		lockedName?: boolean;
	} = $props();

	let step = $state<'pick-mic' | 'recording' | 'naming' | 'saving' | 'done'>('pick-mic');
	let mics = $state<string[]>([]);
	let profileNames = $state<string[]>([]);
	let selectedMic = $state('');
	let clipId = $state('');
	let profileName = $state('');
	let durationS = $state(0);
	let speechS = $state(0);
	let purity = $state(0);
	let error = $state('');
	let modelReady = $state(false);
	let vadReady = $state(false);
	let modelProgress = $state(0);
	let vadProgress = $state(0);
	let unlisten: (() => void) | undefined;
	let unlistenFocus: (() => void) | undefined;

	async function refreshMics() {
		const devices = await invoke<string[]>('scribe_list_input_devices').catch(() => []);
		mics = devices;
		selectedMic = resolveSelectedMic(selectedMic, devices) || devices[0] || '';
	}

	const progress = $derived(Math.min(100, Math.round((speechS / 10) * 100)));
	const safeToStop = $derived(speechS >= 4.5 && purity >= 0.45);
	const purityPct = $derived(Math.round(purity * 100));
	const micTooQuiet = $derived(durationS > 4 && purity < 0.15);
	const statusText = $derived(
		speechS >= 10
			? 'Optimal'
			: safeToStop
				? 'Safe to stop'
				: micTooQuiet
					? 'Mic too quiet'
					: durationS > 0
						? 'Keep speaking'
						: 'Waiting for speech',
	);

	onMount(async () => {
		profileName = prefilledName;
		await refreshMics();
		profileNames = await invoke<string[]>('voiceprint_list_profile_names').catch(() => []);
		if (!lockedName && profileNames.length > 0 && prefilledName !== 'You') {
			profileName = prefilledName;
		}

		unlisten = await listen<ClipStatus>('voiceprint://clip-status', (event) => {
			if (event.payload.clip_id !== clipId) return;
			durationS = event.payload.duration_s;
			speechS = event.payload.speech_s;
			purity = event.payload.purity;
		});

		const status = await invoke<ModelStatus>('voiceprint_model_status').catch(() => ({
			downloaded: false,
			path: '',
		}));
		modelReady = status.downloaded;
		vadReady = await invoke<boolean>('model_vad_status').catch(() => false);
		if (!vadReady) {
			void downloadVad();
		}
		if (!modelReady) {
			void downloadModel();
		}

		unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
			if (focused) void refreshMics();
		});
	});

	onDestroy(() => {
		unlisten?.();
		unlistenFocus?.();
		if (clipId && step !== 'done' && step !== 'saving') {
			void invoke('voiceprint_discard_clip', { clipId }).catch(() => {});
		}
	});

	async function downloadModel() {
		error = '';
		const stop = await listen<{ progress: number }>('voiceprint://model-downloading', (event) => {
			modelProgress = Math.round((event.payload.progress ?? 0) * 100);
			if (event.payload.progress >= 1) modelReady = true;
		});
		try {
			await invoke('voiceprint_download_model');
		} catch (e) {
			error = `Could not start voiceprint model download: ${appErrorMessage(e)}`;
		} finally {
			stop();
		}
	}

	async function downloadVad() {
		error = '';
		const stop = await listen<{ model_id: string; progress: number }>(
			'model://download-progress',
			(event) => {
				if (event.payload.model_id !== 'vad') return;
				vadProgress = Math.round((event.payload.progress ?? 0) * 100);
				if (event.payload.progress >= 1) vadReady = true;
			},
		);
		try {
			await invoke('model_vad_download');
		} catch (e) {
			error = `Could not prepare voice activity detection: ${appErrorMessage(e)}`;
		} finally {
			stop();
		}
	}

	async function startRecording() {
		if (!selectedMic) {
			error = 'Choose a microphone first.';
			return;
		}
		error = '';
		durationS = 0;
		speechS = 0;
		purity = 0;
		try {
			clipId = await invoke<string>('voiceprint_start_clip', { micDeviceId: selectedMic });
			step = 'recording';
		} catch (e) {
			error = `Could not start voiceprint capture: ${appErrorMessage(e)}`;
		}
	}

	async function stopRecording() {
		error = '';
		try {
			const result = await invoke<ClipResult>('voiceprint_stop_clip', { clipId });
			speechS = result.speech_s;
			purity = result.purity;
			if (!result.accepted) {
				error = 'Too noisy or too short — try again.';
				clipId = '';
				step = 'pick-mic';
				return;
			}
			step = 'naming';
		} catch (e) {
			error = `Could not prepare voiceprint: ${appErrorMessage(e)}`;
		}
	}

	async function saveProfile() {
		const name = profileName.trim();
		if (!name) {
			error = 'Profile name cannot be empty.';
			return;
		}
		error = '';
		step = 'saving';
		const id = clipId;
		clipId = '';
		try {
			await invoke('voiceprint_commit_clip', { clipId: id, profileName: name });
			step = 'done';
			if (!isFirstTime) onNext();
		} catch (e) {
			error = `Could not save voiceprint: ${appErrorMessage(e)}`;
			clipId = id;
			step = 'naming';
		}
	}

	async function cancel() {
		if (clipId) {
			await invoke('voiceprint_discard_clip', { clipId }).catch(() => {});
			clipId = '';
		}
		onBack();
	}
</script>

<StepShell
	title={step === 'done' ? 'Voiceprint saved' : 'Enroll a voice'}
	subtitle="Record a short, clean sample so transcripts can label speakers locally."
>
	{#snippet children()}
		<div class="space-y-4">
			{#if error}
				<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
					{error}
				</p>
			{/if}

			{#if !modelReady || !vadReady}
				<div class="space-y-2 rounded-md border border-fill bg-panel p-3">
					<p class="sf-label-md text-fg">Preparing voice tools</p>
					<div class="h-1.5 overflow-hidden rounded-sm bg-fill">
						<div class="h-full bg-brand transition-[width] duration-200" style={`width:${modelProgress}%`}></div>
					</div>
					<p class="sf-label-sm text-fg-dim">Voiceprint model {modelProgress}%</p>
					<div class="h-1.5 overflow-hidden rounded-sm bg-fill">
						<div class="h-full bg-focus transition-[width] duration-200" style={`width:${vadProgress}%`}></div>
					</div>
					<p class="sf-label-sm text-fg-dim">Voice activity detection {vadProgress}%</p>
				</div>
			{:else if step === 'pick-mic'}
				<div class="space-y-2">
					<label class="sf-field-label" for="voice-mic">Which mic should we use?</label>
					<select id="voice-mic" bind:value={selectedMic} class="sf-input h-10 w-full p-2">
						{#each mics as mic (mic)}
							<option value={mic}>{mic}</option>
						{/each}
					</select>
					{#if mics.length === 0}
						<p class="sf-label-sm text-fg-dim">No microphone devices found.</p>
					{/if}
				</div>
			{:else if step === 'recording'}
				<div class="space-y-4">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-2">
							<Mic class="size-4 text-brand" />
							<p class="sf-label-md text-fg">Speak naturally for 10 seconds</p>
						</div>
						<span class="sf-label-sm text-fg-dim tabular-nums">{durationS.toFixed(0)}s</span>
					</div>
					<div class="space-y-2">
						<div class="flex items-center justify-between sf-label-sm text-fg-dim">
							<span>Voice signal {purityPct}%</span>
							<span class={micTooQuiet ? 'text-warning' : ''}>{statusText}</span>
						</div>
						<div class="h-2 overflow-hidden rounded-sm bg-fill">
							<div
								class="h-full transition-[width] duration-200 {micTooQuiet ? 'bg-warning' : 'bg-focus'}"
								style={`width:${purityPct}%`}
							></div>
						</div>
						{#if micTooQuiet}
							<p class="sf-label-sm text-fg-dim">
								Speak louder, move closer to the mic, or raise your input volume in System Settings.
							</p>
						{/if}
					</div>
					<div class="space-y-2">
						<div class="h-2 overflow-hidden rounded-sm bg-fill">
							<div class="h-full bg-brand transition-[width] duration-200" style={`width:${progress}%`}></div>
						</div>
						<div class="flex justify-between sf-meta-sm text-fg-dim">
							<span>0s</span>
							<span>5s safe</span>
							<span>10s optimal</span>
						</div>
					</div>
					<p class="sf-label-sm text-fg-dim">Speech detected: {speechS.toFixed(1)} s</p>
				</div>
			{:else if step === 'naming'}
				<div class="space-y-3">
					<TextField
						label="Who was that?"
						bind:value={profileName}
						disabled={lockedName}
						placeholder="You"
					/>
					{#if !lockedName}
						<datalist id="voice-profile-names">
							{#each profileNames as name (name)}
								<option value={name}></option>
							{/each}
						</datalist>
					{/if}
					<p class="sf-label-sm text-fg-dim">
						Adding to an existing profile makes it more accurate across distances and mics.
					</p>
				</div>
			{:else if step === 'saving'}
				<p class="sf-label-md text-fg-dim">Saving voiceprint…</p>
			{:else}
				<div class="space-y-3">
					<CircleCheckBig class="size-7 text-success" />
					<p class="sf-body-md text-fg">Your transcripts can now label this voice.</p>
					<p class="sf-label-sm text-fg-dim">
						More prints improve accuracy. Add them later in Settings → Voice.
					</p>
				</div>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={cancel}>Cancel</Button>
		<div class="flex items-center gap-2">
			{#if step === 'pick-mic'}
				<Button variant="primary" disabled={!modelReady || !vadReady || !selectedMic} onclick={startRecording}>Next</Button>
			{:else if step === 'recording'}
				<Button variant="primary" onclick={stopRecording}>Stop</Button>
			{:else if step === 'naming'}
				<Button variant="primary" onclick={saveProfile}>Save</Button>
			{:else if step === 'done'}
				<Button variant="primary" onclick={onNext}>Done</Button>
			{/if}
		</div>
	{/snippet}
</StepShell>
