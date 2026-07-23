<script lang="ts">
	import { X, Tag } from 'lucide-svelte';
	import IconButton from '@components/controls/IconButton.svelte';
	import CheckboxGroup from '@components/controls/CheckboxGroup.svelte';
	import FilterCheckboxRow from '@components/cards/FilterRow.svelte';
	import ScrollablePanel from '@primitives/layout/ScrollBody.svelte';
	import type { TagVocabularyEntry } from '@services/historyActions';

	let {
		vocabulary,
		selectedTags,
		activeFilterCount,
		showingCount,
		totalCount,
		onclose,
		ontoggle,
	}: {
		vocabulary: TagVocabularyEntry[];
		selectedTags: Set<string>;
		activeFilterCount: number;
		showingCount: number;
		totalCount: number;
		onclose: () => void;
		ontoggle: (tag: string, checked: boolean) => void;
	} = $props();
</script>

<aside class="flex h-full min-h-0 w-[260px] shrink-0 flex-col border-l border-card bg-panel">
	<div class="flex shrink-0 items-center justify-between border-b border-card p-4">
		<p class="sf-label-md text-fg">Filter</p>
		<IconButton aria-label="Close filter panel" icon={X} size="small" variant="normal" onclick={onclose} />
	</div>

	<ScrollablePanel class="flex flex-col gap-5 p-4">
		<div>
			<p class="sf-section-label mb-2 flex items-center gap-1.5 text-fg-dim">
				<Tag class="size-3" aria-hidden="true" />
				Tags
			</p>
			{#if vocabulary.length === 0}
				<p class="sf-body-md text-fg-muted">
					No vocabulary yet.
				</p>
			{:else}
				<CheckboxGroup>
					{#each vocabulary as entry (entry.name)}
						<FilterCheckboxRow
							label={entry.name}
							count={entry.count}
							checked={selectedTags.has(entry.name)}
							onchange={(next) => ontoggle(entry.name, next)}
						/>
					{/each}
				</CheckboxGroup>
			{/if}
		</div>
	</ScrollablePanel>

	<div class="shrink-0 border-t border-card p-4">
		<p class="sf-meta-sm text-fg-dim">
			{activeFilterCount} active {activeFilterCount === 1 ? 'filter' : 'filters'} · showing {showingCount} of {totalCount}
		</p>
	</div>
</aside>
