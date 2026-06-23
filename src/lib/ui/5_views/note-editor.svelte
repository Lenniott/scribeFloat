<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import EditableTitle from '@components/controls/EditableTitle.svelte';
	import MarkdownEditor from '@components/controls/MarkdownEditor.svelte';
	import Button from '@components/controls/Button.svelte';
	import Modal from '@primitives/layout/Modal.svelte';
	import RecordingStrip from '@sections/RecordingStrip.svelte';
	import TranscriptPanel from '@sections/TranscriptPanel.svelte';
	import { loadNotes } from '@stores/appActions';
	import { appState } from '@stores/appState.svelte';
	import { runNoteLeaveGuard } from '@services/noteLeaveGuard';

	type Segment = { start_ms: number; end_ms: number; text: string };
	type RightPanel = 'transcript' | 'metadata';

	type HistoryRecord = {
		id: string;
		title: string;
		written_content?: string | null;
		segments?: Segment[];
	};

	let { id }: { id: string } = $props();

	let title = $state('');
	let writtenContent = $state('');
	let segments = $state<Segment[]>([]);
	let showTranscript = $state(false);
	let showMetadata = $state(true);
	let transcriptKey = $state(0);
	let titleTimer: ReturnType<typeof setTimeout> | null = null;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let initialized = false;
	/** Last values persisted (or loaded) — autosave skips when unchanged. */
	let savedTitle = '';
	let savedContent = '';
	let recordingActive = $state(false);
	let showDiscardModal = $state(false);
	let pendingProceed: (() => void) | null = null;
	let pendingCancel: (() => void) | null = null;

	const hasTranscript = $derived(segments.length > 0);
	const hasRightPanel = $derived(showTranscript || showMetadata);

	const rightPanelOptions: { id: RightPanel; label: string }[] = [
		{ id: 'transcript', label: 'Transcript' },
		{ id: 'metadata', label: 'Metadata' },
	];

	function defaultRightPanels(hasSegs: boolean, written: string): {
		showTranscript: boolean;
		showMetadata: boolean;
	} {
		if (!hasSegs) return { showTranscript: false, showMetadata: true };
		if (!written.trim()) return { showTranscript: true, showMetadata: false };
		return { showTranscript: false, showMetadata: true };
	}

	/** Written is always visible. At most one right panel (Transcript or Metadata). */
	function toggleRightPanel(panel: RightPanel) {
		if (panel === 'transcript' && !hasTranscript) return;

		if (panel === 'transcript') {
			if (showTranscript) {
				showTranscript = false;
			} else {
				showTranscript = true;
				showMetadata = false;
			}
			return;
		}

		if (showMetadata) {
			showMetadata = false;
		} else {
			showMetadata = true;
			showTranscript = false;
		}
	}

	$effect(() => {
		if (!hasTranscript) {
			showTranscript = false;
		}
	});

	onMount(async () => {
		appState.noteLeaveGuard = (proceed, cancel) => {
			void runNoteLeaveGuard(
				{
					id,
					recordingActive,
					invoke,
					onEmptyDeleted: () => loadNotes(),
				},
				{
					proceed,
					cancel,
					showMetadataDiscard: () => {
						pendingProceed = proceed;
						pendingCancel = cancel;
						showDiscardModal = true;
					},
				},
			);
		};

		try {
			const record = await invoke<HistoryRecord>('history_get_detail', { id });
			title = record.title;
			writtenContent = record.written_content ?? '';
			segments = record.segments ?? [];
			const defaults = defaultRightPanels(segments.length > 0, writtenContent);
			showTranscript = defaults.showTranscript;
			showMetadata = defaults.showMetadata;
		} catch {
			void goto('/notes');
			return;
		}

		savedTitle = title;
		savedContent = writtenContent;
		initialized = true;
	});

	onDestroy(() => {
		if (appState.noteLeaveGuard) {
			appState.noteLeaveGuard = null;
		}
	});

	async function onDiscardEmptyNote() {
		showDiscardModal = false;
		await invoke('history_delete', { id });
		await loadNotes();
		pendingProceed?.();
		pendingProceed = null;
		pendingCancel = null;
	}

	function onKeepEmptyNote() {
		showDiscardModal = false;
		pendingCancel?.();
		pendingProceed = null;
		pendingCancel = null;
	}

	$effect(() => {
		const t = title;
		if (!initialized || t === savedTitle) return;
		if (titleTimer) clearTimeout(titleTimer);
		titleTimer = setTimeout(async () => {
			if (t === savedTitle) return;
			await invoke('note_save_title', { id, title: t });
			savedTitle = t;
		}, 500);
	});

	function onContentChange(v: string) {
		writtenContent = v;
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(async () => {
			if (v === savedContent) return;
			await invoke('note_save_written_content', { id, content: v });
			savedContent = v;
		}, 800);
	}

	async function onTranscriptReady() {
		try {
			const record = await invoke<HistoryRecord>('history_get_detail', { id });
			segments = record.segments ?? [];
			transcriptKey += 1;
			showTranscript = true;
			showMetadata = false;
		} catch {
			// list refresh on next visit still picks up changes
		}
	}
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-panel">
	<div class="flex shrink-0 items-center border-b border-card px-4 py-2">
		<div class="flex items-center gap-2 flex-1">
		<EditableTitle bind:value={title} placeholder="Untitled note" />
		</div>
		<RecordingStrip noteId={id} bind:recordingActive ontranscriptready={onTranscriptReady} />
	</div>



	<div
		class="flex shrink-0 items-center gap-2 border-b border-card px-4 py-2"
		role="toolbar"
		aria-label="Toggle side panels"
	>
		<span class="sf-label-sm mr-1 text-fg-dim">Written</span>
		<span class="text-fg-muted" aria-hidden="true">·</span>
		{#each rightPanelOptions as option (option.id)}
			{@const available = option.id !== 'transcript' || hasTranscript}
			{@const on = option.id === 'transcript' ? showTranscript : showMetadata}
			<button
				type="button"
				disabled={!available}
				aria-pressed={on}
				class="sf-label-sm rounded border px-2.5 py-1 transition-colors disabled:cursor-not-allowed disabled:opacity-40 {on
					? 'border-active bg-active/15 text-fg'
					: 'border-transparent text-fg-dim hover:bg-fill hover:text-fg'}"
				onclick={() => toggleRightPanel(option.id)}
			>
				{option.label}
			</button>
		{/each}
	</div>

	<div class="flex min-h-0 flex-1 overflow-hidden">
		<!-- Written: always on the left -->
		<div
			class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden {hasRightPanel
				? 'border-r border-card'
				: ''}"
		>
			<div class="min-h-0 flex-1 overflow-y-auto">
				<MarkdownEditor value={writtenContent} onchange={onContentChange} />
			</div>
		</div>

		{#if showTranscript && hasTranscript}
			<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
				<div class="min-h-0 flex-1 overflow-hidden">
					{#key transcriptKey}
						<TranscriptPanel noteId={id} />
					{/key}
				</div>
			</div>
		{:else if showMetadata}
			<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
				<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
					<p class="sf-body-md text-fg-muted">Tags, keywords, and Float layers — story 0047.</p>
				</div>
			</div>
		{/if}
	</div>
</div>

<Modal
	open={showDiscardModal}
	title="Discard empty note?"
	description="This note has metadata but no content. Discard it or keep it as an empty note?"
	maxWidthClass="max-w-sm"
	showCloseButton={false}
>
	{#snippet footer()}
		<div class="flex w-full justify-end gap-3">
			<Button variant="normal" onclick={onKeepEmptyNote}>Keep</Button>
			<Button variant="destructive" onclick={() => void onDiscardEmptyNote()}>Discard</Button>
		</div>
	{/snippet}
</Modal>
