<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { Mic, PenLine, Square } from 'lucide-svelte';
	import Button from '@lib/components/Button.svelte';
	import RecordingStatusDot from '@lib/components/audio/RecordingStatusDot.svelte';

	type DictateState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';

	type DictateStateEvent = {
		state: DictateState;
	};

	let {
		onNewNote,
	}: {
		onNewNote?: () => void;
	} = $props();

	let dictateState = $state<DictateState>('IDLE');

	const isRecording = $derived(dictateState === 'RECORDING');
	const isBusy = $derived(dictateState === 'TRANSCRIBING' || dictateState === 'PASTING');

	async function handleDictateClick() {
		if (isBusy) return;
		await invoke('dictate_trigger').catch(() => {});
	}

	onMount(() => {
		void invoke<DictateState>('dictate_get_state').then((state) => {
			dictateState = state;
		});
		const unlistenP = listen<DictateStateEvent>('dictate://state-changed', (event) => {
			dictateState = event.payload.state;
		});
		return async () => (await unlistenP)();
	});
</script>

<header
	class="flex h-10 shrink-0 items-center justify-between border-b border-card bg-panel px-4"
	data-tauri-drag-region
>
	<p class="sf-label-md text-fg-dim" data-tauri-drag-region>ScribeFloat</p>
	<div class="flex items-center gap-2">
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
