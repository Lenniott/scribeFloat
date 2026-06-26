<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { ArrowLeft, Mic, PenLine, Square } from 'lucide-svelte';
	import Button from '@components/controls/Button.svelte';
	import RecordingStatusDot from '@primitives/display/StatusDot.svelte';

	type DictateState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';
	type ScribeState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'ERROR';

	type DictateStateEvent = { state: DictateState };
	type ScribeStateEvent = { state: ScribeState };

	let {
		onNewNote,
		onBack,
		backLabel = 'Notes',
	}: {
		onNewNote?: () => void;
		onBack?: () => void;
		backLabel?: string;
	} = $props();

	let dictateState = $state<DictateState>('IDLE');
	let scribeState = $state<ScribeState>('IDLE');

	const isRecording = $derived(dictateState === 'RECORDING');
	const isBusy = $derived(dictateState === 'TRANSCRIBING' || dictateState === 'PASTING');
	const scribeRecording = $derived(scribeState === 'RECORDING');

	async function handleDictateClick() {
		if (isBusy) return;
		await invoke('dictate_trigger').catch(() => {});
	}

	function returnToRecording() {
		void goto('/');
	}

	onMount(() => {
		void invoke<DictateState>('dictate_get_state').then((state) => {
			dictateState = state;
		});
		void invoke<ScribeStateEvent>('scribe_get_state').then((payload) => {
			scribeState = payload.state;
		}).catch(() => {});
		const unlistenDictateP = listen<DictateStateEvent>('dictate://state-changed', (event) => {
			dictateState = event.payload.state;
		});
		const unlistenScribeP = listen<ScribeStateEvent>('scribe://state-changed', (event) => {
			scribeState = event.payload.state;
		});
		return async () => {
			(await unlistenDictateP)();
			(await unlistenScribeP)();
		};
	});
</script>

<header class="flex h-10 shrink-0 items-center border-b border-card bg-panel px-4">
	<div class="flex shrink-0 items-center">
		{#if onBack}
			<Button variant="ghost" size="small" icon={ArrowLeft} onclick={onBack}>
				{backLabel}
			</Button>
		{/if}
	</div>
	<div class="flex-1" data-tauri-drag-region></div>
	<div class="flex shrink-0 items-center gap-2">
		{#if scribeRecording}
			<button
				onclick={returnToRecording}
				class="flex items-center gap-1.5 rounded-full bg-destructive/10 px-2.5 py-1 sf-label-sm text-destructive hover:bg-destructive/20"
			>
				<RecordingStatusDot status="recording" pulseWhileRecording />
				Recording
			</button>
		{/if}
		{#if isRecording}
			<RecordingStatusDot status="recording" pulseWhileRecording={false} />
		{/if}
		{#if onNewNote}
			<Button variant="normal" size="small" icon={PenLine} onclick={onNewNote}>
				New Note
			</Button>
		{/if}
		<Button
			variant={isRecording ? 'active' : 'normal'}
			size="small"
			icon={isRecording ? Square : Mic}
			disabled={isBusy}
			onclick={handleDictateClick}
		>
			{isBusy ? 'Dictating…' : isRecording ? 'Stop' : 'Dictate'}
		</Button>
	</div>
</header>
