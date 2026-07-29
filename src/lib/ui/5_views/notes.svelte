<script lang="ts">
	import { goto } from '$app/navigation';
	import { ChevronLeft, ChevronRight, SlidersHorizontal } from 'lucide-svelte';
	import HistoryDetailPane from '@sections/NoteDetailPane.svelte';
	import NoteListCard from '@components/cards/NoteCard.svelte';
	import FilterSidePanel from '@sections/FilterPanel.svelte';
	import ScrollablePanel from '@primitives/layout/ScrollBody.svelte';
	import IconButton from '@components/controls/IconButton.svelte';
	import Button from '@components/controls/Button.svelte';
	import type { HistoryListItem } from '@services/historyActions';
	import { fetchTagVocabulary, type TagVocabularyEntry } from '@services/historyActions';

	type CaptureFilter = 'all' | 'scribe' | 'dictate' | 'upload' | 'written';

	const tabs: { id: CaptureFilter; label: string }[] = [
		{ id: 'all', label: 'All' },
		{ id: 'scribe', label: 'Scribe' },
		{ id: 'dictate', label: 'Dictate' },
		{ id: 'upload', label: 'Upload' },
		{ id: 'written', label: 'Written' },
	];

	function isEditorNote(item: HistoryListItem): boolean {
		return (
			item.source === 'store' &&
			!item.id.startsWith('md::') &&
			!item.id.startsWith('dictate::')
		);
	}

	function openNote(item: HistoryListItem) {
		if (isEditorNote(item)) {
			selectedItem = null;
			void goto(`/notes/${item.id}`);
			return;
		}
		selectedItem = item;
	}

	let {
		allItems,
		loading,
		selectedItem = $bindable(null),
		oncopy,
		onopen,
		ondelete,
		onrefresh,
		deleting = false,
	}: {
		allItems: HistoryListItem[];
		loading: boolean;
		selectedItem?: HistoryListItem | null;
		oncopy: (item: HistoryListItem) => void;
		onopen: (item: HistoryListItem) => void;
		ondelete: (item: HistoryListItem) => void;
		onrefresh: () => void;
		deleting?: boolean;
	} = $props();

	let activeTab = $state<CaptureFilter>('all');
	let filterOpen = $state(false);
	let vocabulary = $state<TagVocabularyEntry[]>([]);
	let selectedTags = $state<Set<string>>(new Set());

	const sourceFiltered = $derived(
		activeTab === 'all'
			? allItems
			: activeTab === 'scribe'
				? allItems.filter((item) => item.kind === 'scribe')
				: activeTab === 'dictate'
					? allItems.filter((item) => item.kind === 'dictate')
					: activeTab === 'written'
						? allItems.filter((item) => item.kind === 'written')
						: allItems.filter((item) => item.kind === 'transcribe'),
	);

	const filteredItems = $derived(
		selectedTags.size === 0
			? sourceFiltered
			: sourceFiltered.filter((item) => {
					const tags = item.tags ?? [];
					return tags.some((t) => selectedTags.has(t));
				}),
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

	$effect(() => {
		void fetchTagVocabulary().then((v) => (vocabulary = v));
	});

	function emptyMessage(filter: CaptureFilter): string {
		if (filter === 'scribe') return 'No Scribe notes yet.';
		if (filter === 'dictate') return 'No dictations yet.';
		if (filter === 'upload') return 'No uploads yet.';
		if (filter === 'written') return 'No written notes yet.';
		return 'No notes yet.';
	}

	function chipForKind(kind: string): { label: string; variant: 'brand' | 'focus' | 'muted' } {
		if (kind === 'dictate') return { label: 'Dictate', variant: 'focus' };
		if (kind === 'transcribe') return { label: 'Upload', variant: 'focus' };
		if (kind === 'written') return { label: 'Written', variant: 'muted' };
		return { label: 'Scribe', variant: 'brand' };
	}

	function navigateDetail(delta: -1 | 1) {
		if (selectedIndex < 0) return;
		const next = filteredItems[selectedIndex + delta];
		if (next) selectedItem = next;
	}

	function toggleTag(tag: string, checked: boolean) {
		const next = new Set(selectedTags);
		if (checked) next.add(tag);
		else next.delete(tag);
		selectedTags = next;
	}
</script>

{#if selectedItem}
	<div class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-panel">
		<header class="flex shrink-0 items-center justify-between gap-4 border-b border-card px-4 py-3">
			<button
				type="button"
				class="sf-label-md text-fg-dim hover:text-fg"
				onclick={() => (selectedItem = null)}
			>
				← Notes
			</button>
			<div class="flex items-center gap-1">
				<IconButton
					aria-label="Previous note"
					icon={ChevronLeft}
					size="small"
					variant="normal"
					disabled={!canGoPrev}
					onclick={() => navigateDetail(-1)}
				/>
				<IconButton
					aria-label="Next note"
					icon={ChevronRight}
					size="small"
					variant="normal"
					disabled={!canGoNext}
					onclick={() => navigateDetail(1)}
				/>
			</div>
		</header>
		<HistoryDetailPane
			item={selectedItem}
			{canGoPrev}
			{canGoNext}
			onprev={() => navigateDetail(-1)}
			onnext={() => navigateDetail(1)}
			onclose={() => (selectedItem = null)}
			onrefresh={() => {
				onrefresh();
				if (selectedItem) {
					const refreshed = allItems.find((i) => i.id === selectedItem!.id);
					selectedItem = refreshed ?? null;
				}
			}}
		/>
	</div>
{:else}
	<div class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
		<div class="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden" role="tabpanel">
			<div class="shrink-0 px-6 pt-6">
				<div class="mb-4 flex items-start justify-between gap-4">
					<div>
						<h1 class="sf-headline-sm text-fg">Notes</h1>
						<p class="mt-0.5 sf-body-md text-fg-dim">
							Every note — Scribe, Dictate, Upload, and written.
						</p>
					</div>
					<Button
						variant={filterOpen ? 'active' : 'normal'}
						size="small"
						icon={SlidersHorizontal}
						onclick={() => (filterOpen = !filterOpen)}
					>
						Filter
					</Button>
				</div>

				<div
					class="mb-4 flex gap-1 overflow-x-auto border-b border-card/60"
					role="tablist"
					aria-label="Filter notes by capture method"
				>
					{#each tabs as tab (tab.id)}
						<button
							type="button"
							role="tab"
							aria-selected={activeTab === tab.id}
							class="sf-label-md border-0 border-b-2 px-3 py-1.5 whitespace-nowrap transition-colors {activeTab === tab.id
								? 'border-active bg-active/15 text-fg'
								: 'border-transparent text-fg-dim hover:bg-fill hover:text-fg'}"
							onclick={() => (activeTab = tab.id)}
						>
							{tab.label}
						</button>
					{/each}
				</div>
			</div>

			<ScrollablePanel class="px-6 pb-6">
				{#if loading}
					<p class="sf-body-md text-fg-muted">Loading…</p>
				{:else if filteredItems.length === 0}
					<p class="sf-body-md text-fg-muted">{emptyMessage(activeTab)}</p>
				{:else}
					<div class="flex flex-col gap-2" role="list">
						{#each filteredItems as item (item.id)}
							<div role="listitem">
								<NoteListCard
									{item}
									chip={chipForKind(item.kind)}
									disabled={deleting}
									onselect={() => openNote(item)}
									oncopy={() => oncopy(item)}
									onopen={item.has_markdown && item.markdown_path && item.source !== 'store'
										? () => onopen(item)
										: undefined}
									ondelete={item.source === 'store' ? () => ondelete(item) : undefined}
								/>
							</div>
						{/each}
					</div>
				{/if}
			</ScrollablePanel>
		</div>

		{#if filterOpen}
			<FilterSidePanel
				{vocabulary}
				{selectedTags}
				activeFilterCount={selectedTags.size}
				showingCount={filteredItems.length}
				totalCount={sourceFiltered.length}
				onclose={() => (filterOpen = false)}
				ontoggle={toggleTag}
			/>
		{/if}
	</div>
{/if}
