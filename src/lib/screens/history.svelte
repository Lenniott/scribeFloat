<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import NoteCard, { type Note } from '@lib/components/notes/NoteCard.svelte';
	import Toast from '@lib/components/Toast.svelte';
	import type { ToastState } from '@lib/components/Toast.svelte';
	import SplitPane from '@lib/components/layout/SplitPane.svelte';
	import HistoryDetailPane from '@lib/components/history/HistoryDetailPane.svelte';
	import type { HistoryListItem } from '@lib/components/history/HistoryDetailPane.svelte';
	import { copyTranscript } from '$lib/services/clipboard';

	type FilterTab = 'all' | 'scribe' | 'dictate';

	const tabs: { id: FilterTab; label: string }[] = [
		{ id: 'all', label: 'All' },
		{ id: 'scribe', label: 'Scribe' },
		{ id: 'dictate', label: 'Dictate' },
	];

	let activeTab = $state<FilterTab>('all');
	let allItems = $state<HistoryListItem[]>([]);
	let loading = $state(true);
	let selectedItem = $state<HistoryListItem | null>(null);
	let toastMessage = $state('');
	let toastState = $state<ToastState>('normal');
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;

	const filteredItems = $derived(
		activeTab === 'all'
			? allItems
			: activeTab === 'scribe'
				? allItems.filter((item) => item.kind === 'scribe')
				: allItems.filter((item) => item.kind === 'dictate'),
	);

	function showToast(msg: string, state: ToastState = 'normal') {
		if (toastTimeout) clearTimeout(toastTimeout);
		toastMessage = msg;
		toastState = state;
		toastTimeout = setTimeout(() => {
			toastMessage = '';
			toastTimeout = null;
		}, 2500);
	}

	function formatTimestamp(iso: string): string {
		const date = new Date(iso);
		const now = new Date();
		const isToday =
			date.getFullYear() === now.getFullYear() &&
			date.getMonth() === now.getMonth() &&
			date.getDate() === now.getDate();
		const timeStr = date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
		if (isToday) return timeStr;
		const dateStr = date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
		return `${dateStr}, ${timeStr}`;
	}

	function toNote(id: string, text: string): Note {
		return { id, text, recordedAtMs: 0 };
	}

	function emptyMessage(filter: FilterTab): string {
		if (filter === 'scribe') return 'No Scribe transcripts yet.';
		if (filter === 'dictate') return 'No dictations yet.';
		return 'No history yet.';
	}

	function chipForKind(kind: string): { label: string; variant: 'brand' | 'focus' } {
		if (kind === 'dictate') return { label: 'Dictate', variant: 'focus' };
		if (kind === 'transcribe') return { label: 'Transcribe', variant: 'focus' };
		return { label: 'Scribe', variant: 'brand' };
	}

	async function copyItem(item: HistoryListItem) {
		try {
			if (item.has_markdown && item.markdown_path) {
				await copyTranscript(item.markdown_path);
			} else {
				const text = await invoke<string>('history_render_markdown', { id: item.id });
				await writeText(text);
			}
			showToast('Copied', 'success');
		} catch {
			showToast('Copy failed', 'error');
		}
	}

	async function openItem(item: HistoryListItem) {
		if (!item.markdown_path) return;
		try {
			await invoke('settings_open_transcript', { filePath: item.markdown_path });
		} catch {
			showToast('Could not open file', 'error');
		}
	}

	function selectItem(item: HistoryListItem) {
		selectedItem = item;
	}

	async function loadHistory() {
		loading = true;
		try {
			const items = await invoke<HistoryListItem[]>('history_list');
			// Sort newest first
			allItems = items.sort((a, b) => b.created_at.localeCompare(a.created_at));
		} catch {
			showToast('Failed to load history', 'error');
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		void loadHistory();
		const unlistenP = listen('history://item-added', () => {
			void loadHistory();
		});
		return async () => (await unlistenP)();
	});
</script>

<div class="flex h-screen flex-col overflow-hidden bg-panel">
	<header class="shrink-0 border-b border-card px-4 py-3">
		<p class="font-mono text-label-md tracking-stamped text-fg/80 uppercase">History</p>
	</header>

	<div class="flex min-h-0 flex-1 overflow-hidden">
		<SplitPane>
			{#snippet left()}
				<!-- Tab bar -->
				<div
					class="shrink-0 flex items-center gap-1 overflow-x-auto border-b border-card/60 bg-panel/70 px-3 py-1.5"
					role="tablist"
					aria-label="Filter history"
				>
					{#each tabs as tab (tab.id)}
						<button
							type="button"
							role="tab"
							aria-selected={activeTab === tab.id}
							aria-controls="history-list"
							class="border-0 border-b-2 px-3 py-1.5 text-label-sm font-normal tracking-stamped whitespace-nowrap uppercase transition-colors {activeTab === tab.id
								? 'border-active bg-active/15'
								: 'border-transparent text-fg/70 hover:bg-fill hover:text-fg'}"
							onclick={() => (activeTab = tab.id)}
						>
							{tab.label}
						</button>
					{/each}
				</div>

				<!-- List -->
				<div id="history-list" class="flex-1 overflow-y-auto p-3" role="tabpanel">
					{#if loading}
						<p class="py-6 text-left text-label-md text-fg/45">Loading…</p>
					{:else if filteredItems.length === 0}
						<p class="py-6 text-left text-label-md text-fg/45">{emptyMessage(activeTab)}</p>
					{:else}
						<div class="flex flex-col gap-2" role="list">
							{#each filteredItems as item (item.id)}
								<div role="listitem">
									<NoteCard
										note={toNote(item.id, item.title || item.id)}
										selected={selectedItem?.id === item.id}
										timestampLabel={formatTimestamp(item.created_at)}
										chip={chipForKind(item.kind)}
										onselect={() => selectItem(item)}
										oncopy={() => copyItem(item)}
										onopen={item.has_markdown && item.markdown_path
											? () => openItem(item)
											: undefined}
									/>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/snippet}

			{#snippet right()}
				{#if selectedItem}
					<HistoryDetailPane
						item={selectedItem}
						onclose={() => (selectedItem = null)}
						onrefresh={() => {
							void loadHistory().then(() => {
								// Re-sync selectedItem so chips/paths update after export/delete
								if (selectedItem) {
									const refreshed = allItems.find((i) => i.id === selectedItem!.id);
									selectedItem = refreshed ?? null;
								}
							});
						}}
					/>
				{:else}
					<div class="flex flex-1 items-center justify-center p-6">
						<p class="text-label-md text-fg/35">Select an item to view details</p>
					</div>
				{/if}
			{/snippet}
		</SplitPane>
	</div>
</div>

<Toast message={toastMessage} state={toastState} position="bottom-center" />
