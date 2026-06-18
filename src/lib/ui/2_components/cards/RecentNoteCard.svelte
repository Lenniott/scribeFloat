<script lang="ts">
	import Chip from '@primitives/display/Chip.svelte';
	import SourceKindIcon from '@primitives/display/SourceIcon.svelte';
	import type { HistoryListItem } from '@services/historyActions';
	import {
		formatDurationFromSecs,
		formatShortDate,
		kindLabel,
	} from '@services/historyFormat';

	let {
		item,
		onselect,
	}: {
		item: HistoryListItem;
		onselect: () => void;
	} = $props();

	const metaLine = $derived(
		`${kindLabel(item.kind)} · ${formatShortDate(item.created_at)} · ${formatDurationFromSecs(item.duration_secs)}`,
	);
</script>

<button
	type="button"
	class="w-full rounded-md border border-fill bg-card p-4 text-left transition-colors hover:bg-fill/80"
	onclick={onselect}
>
	<div class="flex items-start gap-3">
		<SourceKindIcon kind={item.kind} />
		<div class="min-w-0 flex-1">
			<div class="flex items-start justify-between gap-3">
				<p class="truncate sf-body-md-strong text-fg">{item.title || item.id}</p>
				<span class="sf-meta-sm shrink-0 text-fg-dim">{metaLine}</span>
			</div>
			{#if item.excerpt}
				<p class="mt-0.5 truncate sf-body-md text-fg-dim">{item.excerpt}</p>
			{/if}
			{#if item.tags && item.tags.length > 0}
				<div class="mt-2 flex flex-wrap gap-1.5">
					{#each item.tags as tag (tag)}
						<Chip variant="muted">{tag}</Chip>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</button>
