<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import NoteCard, { type Note } from '@lib/components/notes/NoteCard.svelte';
	import Toast from '@lib/components/Toast.svelte';
	import type { ToastState } from '@lib/components/Toast.svelte';
	import { copyTranscript } from '$lib/services/clipboard';

	type DictateEntry = { kind: 'dictate'; id: string; timestamp: string; text: string };
	type ScribeEntry = { kind: 'scribe'; path: string; title: string; model: string; modifiedAt: string };
	type HistoryItem = DictateEntry | ScribeEntry;
	type FilterTab = 'all' | 'scribe' | 'dictate';

	const tabs: { id: FilterTab; label: string }[] = [
		{ id: 'all', label: 'All' },
		{ id: 'scribe', label: 'Scribe' },
		{ id: 'dictate', label: 'Dictate' },
	];

	let activeTab = $state<FilterTab>('all');
	let allItems = $state<HistoryItem[]>([]);
	let loading = $state(true);
	let toastMessage = $state('');
	let toastState = $state<ToastState>('normal');
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;

	const filteredItems = $derived(
		activeTab === 'all'
			? allItems
			: allItems.filter((item) => item.kind === activeTab),
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

	async function copyDictate(text: string) {
		try {
			await writeText(text);
			showToast('Copied', 'success');
		} catch {
			showToast('Copy failed', 'error');
		}
	}

	async function copyScribe(path: string) {
		try {
			await copyTranscript(path);
			showToast('Copied', 'success');
		} catch {
			showToast('Copy failed', 'error');
		}
	}

	async function openScribe(path: string) {
		try {
			await invoke('settings_open_transcript', { filePath: path });
		} catch {
			showToast('Could not open file', 'error');
		}
	}

	onMount(async () => {
		try {
			type RawDictate = { id: string; timestamp: string; text: string };
			type RawScribe = { path: string; title: string; model: string; modified_at: string };

			const [dictateRaw, scribeRaw] = await Promise.all([
				invoke<RawDictate[]>('dictate_get_history'),
				invoke<RawScribe[]>('scribe_list_transcripts'),
			]);

			const dictateItems: HistoryItem[] = dictateRaw.map((e) => ({
				kind: 'dictate',
				id: e.id,
				timestamp: e.timestamp,
				text: e.text,
			}));

			const scribeItems: HistoryItem[] = scribeRaw.map((e) => ({
				kind: 'scribe',
				path: e.path,
				title: e.title,
				model: e.model,
				modifiedAt: e.modified_at,
			}));

			allItems = [...dictateItems, ...scribeItems].sort((a, b) => {
				const ta = a.kind === 'dictate' ? a.timestamp : a.modifiedAt;
				const tb = b.kind === 'dictate' ? b.timestamp : b.modifiedAt;
				return tb.localeCompare(ta);
			});
		} catch {
			showToast('Failed to load history', 'error');
		} finally {
			loading = false;
		}
	});
</script>

<!-- Three flex siblings: header (shrink-0), tab bar (shrink-0), list (flex-1 scrollable) -->
<div class="flex h-screen flex-col overflow-hidden bg-panel">
	<header class="shrink-0 border-b border-card px-4 py-3">
		<p class="font-mono text-label-md tracking-stamped text-fg/80 uppercase">History</p>
	</header>

	<div
		class="shrink-0 flex items-center gap-1 overflow-x-auto border-b border-card/60 bg-panel/70 px-1.5 py-1.5"
		role="tablist"
		aria-label="Filter history"
	>
		{#each tabs as tab (tab.id)}
			<button
				type="button"
				role="tab"
				aria-selected={activeTab === tab.id}
				aria-controls="history-list"
				class="border-0 border-b-2 px-2.5 py-1.5 text-label-sm font-normal tracking-stamped whitespace-nowrap uppercase transition-colors {activeTab === tab.id
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
				{#each filteredItems as item (item.kind === 'dictate' ? item.id : item.path)}
					<div role="listitem">
						{#if item.kind === 'dictate'}
							<NoteCard
								note={toNote(item.id, item.text)}
								timestampLabel={formatTimestamp(item.timestamp)}
								chip={{ label: 'Dictate', variant: 'focus' }}
								oncopy={() => copyDictate(item.text)}
							/>
						{:else}
							<NoteCard
								note={toNote(item.path, item.title)}
								timestampLabel={formatTimestamp(item.modifiedAt)}
								chip={{ label: 'Scribe', variant: 'brand' }}
								oncopy={() => copyScribe(item.path)}
								onopen={() => openScribe(item.path)}
							/>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<Toast message={toastMessage} state={toastState} position="bottom-center" />
