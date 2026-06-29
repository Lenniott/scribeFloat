<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { ArrowLeft, Mic, PenLine, Square, Trash2 } from 'lucide-svelte';
	import Button from '@components/controls/Button.svelte';
	import IconButton from '@components/controls/IconButton.svelte';
	import Modal from '@primitives/layout/Modal.svelte';
	import RecordingStatusDot from '@primitives/display/StatusDot.svelte';
	import RecordingTimer from '@primitives/display/RecordingTimer.svelte';
	import { appState } from '@stores/appState.svelte';

	type DictateState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';
	type DictateStateEvent = { state: DictateState };
	type ScribePhase = 'idle' | 'recording' | 'transcribing';
	type ScribePayload = { state: string; error?: string };

	let {
		onNewNote,
		onBack,
		backLabel = 'Back',
	}: {
		onNewNote?: () => void;
		onBack?: () => void;
		backLabel?: string;
	} = $props();

	let dictateState = $state<DictateState>('IDLE');
	let scribePhase = $state<ScribePhase>('idle');
	let scribeElapsedMs = $state(0);
	let scribeTimerInterval: ReturnType<typeof setInterval> | null = null;
	let showDiscardConfirm = $state(false);

	const isRecording = $derived(dictateState === 'RECORDING');
	const isBusy = $derived(dictateState === 'TRANSCRIBING' || dictateState === 'PASTING');
	const dictateDisabled = $derived(isBusy || scribePhase !== 'idle');

	const isOnRecordingNote = $derived(
		appState.scribeNoteId !== null &&
		page.url.pathname === `/notes/${appState.scribeNoteId}`
	);
	const showScribeControls = $derived(scribePhase !== 'idle' && !isOnRecordingNote);
	const scribeElapsedSeconds = $derived(Math.floor(scribeElapsedMs / 1000));

	function startScribeTimer() {
		if (scribeTimerInterval) clearInterval(scribeTimerInterval);
		const start = Date.now();
		scribeElapsedMs = 0;
		scribeTimerInterval = setInterval(() => { scribeElapsedMs = Date.now() - start; }, 100);
	}

	function stopScribeTimer() {
		if (scribeTimerInterval) { clearInterval(scribeTimerInterval); scribeTimerInterval = null; }
	}

	async function scribeStopAndSave() {
		scribePhase = 'transcribing';
		stopScribeTimer();
		appState.scribeAwaitingAttach = true;
		await invoke('scribe_stop_and_save', { title: null }).catch(() => {
			scribePhase = 'idle';
			appState.scribeAwaitingAttach = false;
		});
	}

	async function scribeDiscard() {
		showDiscardConfirm = false;
		stopScribeTimer();
		await invoke('scribe_set_attach_note', { noteId: null }).catch(() => {});
		await invoke('scribe_cancel').catch(() => {});
		scribePhase = 'idle';
		appState.scribeNoteId = null;
		appState.scribeAwaitingAttach = false;
	}

	function handleScribeEvent(payload: ScribePayload) {
		switch (payload.state) {
			case 'IDLE':
				scribePhase = 'idle';
				stopScribeTimer();
				appState.scribeNoteId = null;
				break;
			case 'RECORDING':
				if (scribePhase !== 'recording') {
					scribePhase = 'recording';
					startScribeTimer();
				}
				break;
			case 'TRANSCRIBING':
				scribePhase = 'transcribing';
				stopScribeTimer();
				break;
			case 'DONE':
				if (appState.scribeAwaitingAttach && !isOnRecordingNote) {
					const noteId = appState.scribeNoteId;
					appState.scribeAwaitingAttach = false;
					appState.scribeNoteId = null;
					scribePhase = 'idle';
					if (noteId) void invoke('note_attach_transcript', { id: noteId });
				}
				break;
			case 'ERROR':
				scribePhase = 'idle';
				stopScribeTimer();
				appState.scribeNoteId = null;
				appState.scribeAwaitingAttach = false;
				break;
		}
	}

	async function handleDictateClick() {
		if (dictateDisabled) return;
		await invoke('dictate_trigger').catch(() => {});
	}

	function handleRecordClick() {
		if (!onNewNote) return;
		appState.scribeAutoStart = true;
		onNewNote();
	}

	onMount(() => {
		void invoke<DictateState>('dictate_get_state').then((state) => {
			dictateState = state;
		});
		const unlistenDictateP = listen<DictateStateEvent>('dictate://state-changed', (event) => {
			dictateState = event.payload.state;
		});
		const unlistenScribeP = listen<ScribePayload>('scribe://state-changed', (event) => {
			handleScribeEvent(event.payload);
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
		{#if showScribeControls}
			{#if scribePhase === 'recording'}
				<RecordingStatusDot status="recording" />
				<RecordingTimer elapsedSeconds={scribeElapsedSeconds} />
				{#if appState.scribeNoteId}
					<Button variant="ghost" size="small" onclick={() => void goto(`/notes/${appState.scribeNoteId}`)}>
						Go to note
					</Button>
				{/if}
				<Button variant="normal" size="small" onclick={scribeStopAndSave}>Stop & Save</Button>
				<IconButton aria-label="Discard recording" icon={Trash2} size="small" variant="normal" onclick={() => (showDiscardConfirm = true)} />
			{:else}
				<span class="sf-body-sm text-fg-dim">Transcribing…</span>
			{/if}
		{/if}
		{#if isRecording}
			<RecordingStatusDot status="recording" pulseWhileRecording={false} />
		{/if}
		{#if scribePhase === 'idle' && onNewNote}
			<Button variant="normal" size="small" icon={PenLine} onclick={handleRecordClick}>
				Record
			</Button>
		{/if}
		<Button
			variant={isRecording ? 'active' : 'normal'}
			size="small"
			icon={isRecording ? Square : Mic}
			disabled={dictateDisabled}
			onclick={handleDictateClick}
		>
			{isBusy ? 'Dictating…' : isRecording ? 'Stop' : 'Dictate'}
		</Button>
	</div>
</header>

<Modal
	open={showDiscardConfirm}
	title="Discard recording?"
	description="The recording will be permanently lost. This cannot be undone."
	maxWidthClass="max-w-sm"
	onClose={() => (showDiscardConfirm = false)}
>
	{#snippet footer()}
		<div class="flex w-full justify-end gap-3">
			<Button variant="normal" onclick={() => (showDiscardConfirm = false)}>Cancel</Button>
			<Button variant="destructive" onclick={() => void scribeDiscard()}>Discard</Button>
		</div>
	{/snippet}
</Modal>
