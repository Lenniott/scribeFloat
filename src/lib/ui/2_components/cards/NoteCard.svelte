<script lang="ts">
	import { Copy, Eye, SquareArrowOutUpRight, Trash2 } from 'lucide-svelte';
	import IconButton from '../controls/IconButton.svelte';
	import Chip from '../../primitives/display/Chip.svelte';
	import SourceKindIcon from '../../primitives/display/SourceIcon.svelte';
	import type { HistoryListItem } from '@services/historyActions';
	import {
		formatDurationFromSecs,
		formatShortDate,
		kindLabel,
	} from '@services/historyFormat';

	let {
		item,
		chip,
		disabled = false,
		onselect,
		oncopy,
		onopen,
		ondelete,
	}: {
		item: HistoryListItem;
		chip: { label: string; variant: 'brand' | 'focus' | 'muted' };
		disabled?: boolean;
		onselect?: () => void;
		oncopy?: () => void;
		onopen?: () => void;
		ondelete?: () => void;
	} = $props();

	const showOpen = $derived(item.has_markdown && !!item.markdown_path);
	const showDelete = $derived(item.source === 'store');
	const isLegacy = $derived(item.source !== 'store');
	const metaLine = $derived(
		`${kindLabel(item.kind)} · ${formatShortDate(item.created_at)} · ${formatDurationFromSecs(item.duration_secs)}`,
	);
</script>

<article
	class="rounded-md border border-fill bg-card p-4 text-left transition-colors hover:bg-fill/80 {disabled
		? 'pointer-events-none opacity-50'
		: ''}"
>
	<div class="flex items-start gap-3">
		<SourceKindIcon kind={item.kind} />
		<div class="min-w-0 flex-1">
			<div class="flex items-baseline justify-between gap-2">
				{#if onselect}
					<button
						type="button"
						class="min-w-0 flex-1 truncate text-left sf-body-md-strong text-fg underline-offset-2 hover:underline"
						disabled={disabled}
						onclick={() => onselect()}
					>
						{item.title || item.id}
					</button>
				{:else}
					<p class="truncate sf-body-md-strong text-fg">{item.title || item.id}</p>
				{/if}
				<div class="flex shrink-0 items-center gap-0.5">
					{#if onselect}
						<IconButton
							aria-label="View note"
							icon={Eye}
							size="small"
							variant="normal"
							{disabled}
							onclick={(e) => {
								e.stopPropagation();
								onselect();
							}}
						/>
					{/if}
					{#if oncopy}
						<IconButton
							aria-label="Copy to clipboard"
							icon={Copy}
							size="small"
							variant="normal"
							{disabled}
							onclick={(e) => {
								e.stopPropagation();
								oncopy();
							}}
						/>
					{/if}
					{#if showOpen && onopen}
						<IconButton
							aria-label="Open Markdown file"
							icon={SquareArrowOutUpRight}
							size="small"
							variant="normal"
							{disabled}
							onclick={(e) => {
								e.stopPropagation();
								onopen();
							}}
						/>
					{/if}
					{#if showDelete && ondelete}
						<IconButton
							aria-label="Delete note"
							icon={Trash2}
							size="small"
							variant="destructive"
							{disabled}
							onclick={(e) => {
								e.stopPropagation();
								ondelete();
							}}
						/>
					{/if}
				</div>
			</div>
			{#if item.excerpt}
				<p class="mt-0.5 truncate sf-body-md text-fg-dim">{item.excerpt}</p>
			{/if}
			<div class="mt-1.5 flex flex-wrap items-center gap-2">
				<span class="sf-meta-sm text-fg-dim">{metaLine}</span>
				<Chip variant={chip.variant}>{chip.label}</Chip>
				{#if isLegacy}
					<span class="sf-label-sm text-fg-muted">Legacy</span>
				{/if}
				{#if item.tags && item.tags.length > 0}
					{#each item.tags.slice(0, 3) as tag (tag)}
						<Chip variant="muted">{tag}</Chip>
					{/each}
				{/if}
			</div>
		</div>
	</div>
</article>
