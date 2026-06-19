<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';
	import EditableTitle from '@components/controls/EditableTitle.svelte';
	import MarkdownEditor from '@components/controls/MarkdownEditor.svelte';

	type HistoryRecord = {
		id: string;
		title: string;
		written_content?: string | null;
	};

	let { id, registerLeaveGuard }: { id: string; registerLeaveGuard?: (fn: (proceed: () => void) => void) => void } = $props();

	let title = $state('');
	let writtenContent = $state('');
	let titleTimer: ReturnType<typeof setTimeout> | null = null;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	// Non-reactive flag — changing it does not re-run effects
	let initialized = false;

	onMount(async () => {
		registerLeaveGuard?.((proceed) => proceed());

		try {
			const record = await invoke<HistoryRecord>('history_get_detail', { id });
			title = record.title;
			writtenContent = record.written_content ?? '';
		} catch {
			void goto('/notes');
			return;
		}

		initialized = true;
	});

	// Autosave title after user edits (not on initial load)
	$effect(() => {
		const t = title;
		if (!initialized) return;
		if (titleTimer) clearTimeout(titleTimer);
		titleTimer = setTimeout(async () => {
			await invoke('note_save_title', { id, title: t });
		}, 500);
	});

	function onContentChange(v: string) {
		writtenContent = v;
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(async () => {
			await invoke('note_save_written_content', { id, content: v });
		}, 800);
	}
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-panel">
	<!-- Header -->
	<div class="flex shrink-0 items-center gap-3 border-b border-card px-4 py-2">
		<button
			type="button"
			class="sf-label-md shrink-0 text-fg-dim hover:text-fg"
			onclick={() => void goto('/notes')}
		>
			← Notes
		</button>
		<EditableTitle bind:value={title} placeholder="Untitled note" />
	</div>

	<!-- Recording chrome strip placeholder (story 0046) -->
	<div class="shrink-0" style="min-height: 2.5rem;"></div>

	<!-- Panel row -->
	<div class="flex min-h-0 flex-1 overflow-hidden">
		<!-- Left panel: written source editor -->
		<div class="min-h-0 min-w-0 flex-1 overflow-y-auto">
			<MarkdownEditor bind:value={writtenContent} onchange={onContentChange} />
		</div>
		<!-- Right panel: transcript / Float output (story 0047) -->
		<div class="min-h-0 min-w-0 flex-1 overflow-y-auto border-l border-card"></div>
	</div>
</div>
