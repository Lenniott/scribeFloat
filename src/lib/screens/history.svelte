<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import Toast from '@lib/components/Toast.svelte';
	import type { ToastState } from '@lib/components/Toast.svelte';
	import HistoryDetailPane from '@lib/components/history/HistoryDetailPane.svelte';
	import HistoryListCard from '@lib/components/history/HistoryListCard.svelte';
	import Button from '@lib/components/Button.svelte';
	import Modal from '@lib/components/Modal.svelte';
	import {
		copyHistoryItem,
		deleteHistoryItem,
		openHistoryMarkdown,
		type HistoryListItem,
	} from '@lib/services/historyActions';

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

	let deleteTarget = $state<HistoryListItem | null>(null);
	let deleting = $state(false);

	const filteredItems = $derived(
		activeTab === 'all'
			? allItems
			: activeTab === 'scribe'
				? allItems.filter((item) => item.kind === 'scribe')
				: allItems.filter((item) => item.kind === 'dictate'),
	);

	const selectedIndex = $derived(
		selectedItem ? filteredItems.findIndex((i) => i.id === selectedItem!.id) : -1,
	);

	const canGoPrev = $derived(selectedIndex > 0);
	const canGoNext = $derived(
		selectedIndex >= 0 && selectedIndex < filteredItems.length - 1,
	);

	$effect(() => {
		if (!selectedItem) return;
		const inFilter = filteredItems.some((i) => i.id === selectedItem!.id);
		if (!inFilter) {
			selectedItem = null;
		}
	});

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
			await copyHistoryItem(item);
			showToast('Copied', 'success');
		} catch {
			showToast('Copy failed', 'error');
		}
	}

	async function openItem(item: HistoryListItem) {
		if (!item.markdown_path) return;
		try {
			await openHistoryMarkdown(item.markdown_path);
		} catch {
			showToast('Could not open file', 'error');
		}
	}

	function selectItem(item: HistoryListItem) {
		selectedItem = item;
	}

	function navigateDetail(delta: -1 | 1) {
		if (selectedIndex < 0) return;
		const next = filteredItems[selectedIndex + delta];
		if (next) selectedItem = next;
	}

	function requestDelete(item: HistoryListItem) {
		deleteTarget = item;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		deleting = true;
		try {
			await deleteHistoryItem(deleteTarget.id);
			if (selectedItem?.id === deleteTarget.id) {
				selectedItem = null;
			}
			deleteTarget = null;
			await loadHistory();
			showToast('Deleted', 'success');
		} catch (e) {
			showToast('Delete failed: ' + String(e), 'error');
			deleteTarget = null;
		} finally {
			deleting = false;
		}
	}

	async function loadHistory() {
		loading = true;
		try {
			const items = await invoke<HistoryListItem[]>('history_list');
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
		{#if selectedItem}
			<div class="flex min-h-0 min-w-0 flex-1 flex-col bg-panel">
				<HistoryDetailPane
					item={selectedItem}
					{canGoPrev}
					{canGoNext}
					onprev={() => navigateDetail(-1)}
					onnext={() => navigateDetail(1)}
					onclose={() => (selectedItem = null)}
					onrefresh={() => {
						void loadHistory().then(() => {
							if (selectedItem) {
								const refreshed = allItems.find((i) => i.id === selectedItem!.id);
								selectedItem = refreshed ?? null;
							}
						});
					}}
				/>
			</div>
		{:else}
			<div class="flex min-h-0 min-w-0 flex-1 flex-col bg-card">
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

				<div id="history-list" class="flex-1 overflow-y-auto p-3" role="tabpanel">
					{#if loading}
						<p class="py-6 text-left text-label-md text-fg/45">Loading…</p>
					{:else if filteredItems.length === 0}
						<p class="py-6 text-left text-label-md text-fg/45">{emptyMessage(activeTab)}</p>
					{:else}
						<div class="flex flex-col gap-2" role="list">
							{#each filteredItems as item (item.id)}
								<div role="listitem">
									<HistoryListCard
										{item}
										selected={false}
										timestampLabel={formatTimestamp(item.created_at)}
										chip={chipForKind(item.kind)}
										disabled={deleting}
										onselect={() => selectItem(item)}
										oncopy={() => copyItem(item)}
										onopen={item.has_markdown && item.markdown_path
											? () => openItem(item)
											: undefined}
										ondelete={item.source === 'store' ? () => requestDelete(item) : undefined}
									/>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</div>

<Modal
	open={deleteTarget !== null}
	title="Delete recording?"
	description="This will permanently delete the transcript and any associated audio. This cannot be undone."
	maxWidthClass="max-w-sm"
	onClose={() => (deleteTarget = null)}
>
	{#snippet footer()}
		<div class="flex gap-3">
			<Button variant="normal" disabled={deleting} onclick={() => (deleteTarget = null)}>Cancel</Button>
			<Button variant="destructive" disabled={deleting} onclick={() => void confirmDelete()}>
				{deleting ? 'Deleting…' : 'Delete'}
			</Button>
		</div>
	{/snippet}
</Modal>

<Toast message={toastMessage} state={toastState} position="bottom-center" />
