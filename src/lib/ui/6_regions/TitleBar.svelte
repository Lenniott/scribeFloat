<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { ArrowLeft, Mic, Settings2, Square, Trash2 } from 'lucide-svelte';
	import Button from '@components/controls/Button.svelte';
	import IconButton from '@components/controls/IconButton.svelte';
	import ToggleSwitch from '@components/controls/Toggle.svelte';
	import FieldRow from '@primitives/form/FieldRow.svelte';
	import Waveform from '@components/indicators/Waveform.svelte';
	import Modal from '@primitives/layout/Modal.svelte';
	import RecordingStatusDot from '@primitives/display/StatusDot.svelte';
	import RecordingTimer from '@primitives/display/RecordingTimer.svelte';
	import ProgressBar from '@primitives/display/ProgressBar.svelte';
	import { appState } from '@stores/appState.svelte';
	import { scribe } from '@stores/scribeController.svelte';

	type DictateState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';
	type DictateStateEvent = { state: DictateState };

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
	let showDiscardConfirm = $state(false);
	let settingsOpen = $state(false);
	let settingsAnchor: HTMLDivElement | undefined = $state();

	const isRecording = $derived(dictateState === 'RECORDING');
	const isBusy = $derived(dictateState === 'TRANSCRIBING' || dictateState === 'PASTING');
	const dictateDisabled = $derived(isBusy || scribe.phase !== 'idle');

	const scribeProgressSequence = [
		{ label: 'Loading model', complete: scribe.processingStage !== 'LOADING_MODEL' },
		{
			label: 'Transcribing',
			complete: scribe.processingStage === 'WRITING_TRANSCRIPT' ||
				scribe.processingStage === 'CLEANING_UP_AUDIO',
		},
		{
			label: 'Writing transcript',
			complete: scribe.processingStage === 'CLEANING_UP_AUDIO',
		},
	];

	const isOnRecordingNote = $derived(
		appState.scribeNoteId !== null &&
			page.url.pathname === `/notes/${appState.scribeNoteId}`,
	);

	function handleRecordClick() {
		if (scribe.phase !== 'idle') return;
		const match = page.url.pathname.match(/^\/notes\/([^/]+)$/);
		const noteId = match?.[1];
		if (noteId && noteId !== 'new') {
			void scribe.startRecording(noteId);
			return;
		}
		if (!onNewNote) return;
		appState.scribeAutoStart = true;
		onNewNote();
	}

	async function handleDictateClick() {
		if (dictateDisabled) return;
		await invoke('dictate_trigger').catch(() => {});
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

	onMount(() => {
		void invoke<DictateState>('dictate_get_state').then((state) => {
			dictateState = state;
		});
		const unlistenDictateP = listen<DictateStateEvent>('dictate://state-changed', (event) => {
			dictateState = event.payload.state;
		});
		return async () => {
			(await unlistenDictateP)();
		};
	});
</script>

<svelte:window onclick={handleWindowClick} />

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
		{#if scribe.phase === 'recording'}
			<Waveform
				micLevel={scribe.audioLevel}
				speakerLevel={scribe.speakerLevel}
				speakerEnabled={scribe.captureSpeaker}
				size="small"
			/>
			<RecordingStatusDot status="recording" />
			<RecordingTimer elapsedSeconds={scribe.elapsedSeconds} />
			{#if appState.scribeNoteId && !isOnRecordingNote}
				<Button
					variant="ghost"
					size="small"
					onclick={() => void goto(`/notes/${appState.scribeNoteId}`)}
				>
					Go to note
				</Button>
			{/if}
			<Button variant="normal" size="small" onclick={() => void scribe.stopAndSave()}>
				Stop & Save
			</Button>
			<IconButton
				aria-label="Discard recording"
				icon={Trash2}
				size="small"
				variant="normal"
				onclick={() => (showDiscardConfirm = true)}
			/>
		{:else if scribe.phase === 'transcribing'}
			<ProgressBar
				progress={scribe.progressPercent}
				variant="small"
				sequence={scribeProgressSequence}
				indeterminate={scribe.processingStage === 'LOADING_MODEL'}
			/>
		{/if}

		{#if onNewNote || scribe.phase !== 'idle'}
			<div class="relative" bind:this={settingsAnchor}>
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
						class="absolute right-0 top-full z-50 mt-1 w-64 space-y-4 rounded-md border border-fill bg-card p-3 shadow-lg"
						role="group"
						aria-label="Recording settings"
					>
						<FieldRow
							label="Microphone"
							id="titlebar-recording-mic"
							mode="select"
							options={scribe.micOptions}
							bind:value={scribe.selectedMic}
							disabled={scribe.captureSettingsLocked}
							onchange={(v) => void scribe.setMic(v)}
						/>
						<ToggleSwitch
							label="Speaker capture"
							bind:checked={scribe.captureSpeaker}
							disabled={scribe.captureSettingsLocked}
							onchange={(v) => {
								void scribe.setSpeakerCapture(v).catch(() => {
									scribe.captureSpeaker = !v;
								});
							}}
						/>
						<ToggleSwitch
							label="Timestamps"
							checked={scribe.includeTimestamps}
							onchange={(v) => {
								scribe.includeTimestamps = v;
								void invoke('scribe_set_include_timestamps', { enabled: v });
							}}
						/>
					</div>
				{/if}
			</div>
		{/if}

		{#if isRecording}
			<RecordingStatusDot status="recording" pulseWhileRecording={false} />
		{/if}
		{#if scribe.phase === 'idle' && onNewNote}
			<Button variant="normal" size="small" onclick={handleRecordClick}>Record</Button>
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
			<Button
				variant="destructive"
				onclick={() => {
					showDiscardConfirm = false;
					void scribe.discard();
				}}
			>
				Discard
			</Button>
		</div>
	{/snippet}
</Modal>
