<script lang="ts">
	import { Copy, Eye, SquareArrowOutUpRight, Trash2 } from 'lucide-svelte';
	import IconButton from '@lib/components/IconButton.svelte';
	import Chip, { type ChipVariant } from '@lib/components/Chip.svelte';
	import type { HistoryListItem } from '@lib/services/historyActions';

	let {
		item,
		selected = false,
		timestampLabel,
		chip,
		disabled = false,
		onselect,
		oncopy,
		onopen,
		ondelete,
	}: {
		item: HistoryListItem;
		selected?: boolean;
		timestampLabel: string;
		chip: { label: string; variant: ChipVariant };
		disabled?: boolean;
		onselect?: () => void;
		oncopy?: () => void;
		onopen?: () => void;
		ondelete?: () => void;
	} = $props();

	const showOpen = $derived(item.has_markdown && !!item.markdown_path);
	const showDelete = $derived(item.source === 'store');
	const isLegacy = $derived(item.source !== 'store');
</script>

<article
	class="group rounded-md px-3 py-2 text-left transition-colors {selected
		? 'bg-fill'
		: 'bg-card hover:bg-fill/80 group-hover:bg-fill/80'} {disabled ? 'opacity-50 pointer-events-none' : ''}"
>
	<div class="flex flex-col items-start gap-2">
		<div class="flex w-full justify-between items-center gap-2">
			<div class="flex min-w-0 items-center gap-4">
				<Chip variant={chip.variant}>{chip.label}</Chip>
				<span
					class="font-mono text-label-sm font-normal tabular-nums tracking-stamped text-fg/55 shrink-0"
				>
					{timestampLabel}
				</span>
				{#if isLegacy}
					<span class="font-mono text-label-sm tracking-stamped text-fg/45 uppercase shrink-0"
						>Legacy</span
					>
				{/if}
			</div>
			<div class="flex shrink-0 items-center gap-0.5">
				{#if onselect}
					<IconButton
						aria-label="View transcript"
						icon={Eye}
						size="small"
						variant="normal"
						disabled={disabled}
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
						disabled={disabled}
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
						disabled={disabled}
						onclick={(e) => {
							e.stopPropagation();
							onopen();
						}}
					/>
				{/if}
				{#if showDelete && ondelete}
					<IconButton
						aria-label="Delete recording"
						icon={Trash2}
						size="small"
						variant="destructive"
						disabled={disabled}
						onclick={(e) => {
							e.stopPropagation();
							ondelete();
						}}
					/>
				{/if}
			</div>
		</div>
		{#if onselect}
			<button
				type="button"
				class="w-full min-w-0 cursor-pointer rounded-sm px-1 py-0.5 text-left transition-colors hover:bg-fill/60 focus:outline-none focus:ring-2 focus:ring-focus focus:ring-offset-2 focus:ring-offset-card"
				disabled={disabled}
				onclick={() => onselect()}
			>
				<p
					class="truncate text-body-md text-fg/85 underline-offset-2 transition-colors group-hover:text-fg hover:text-fg hover:underline"
					title={item.title || item.id}
				>
					{item.title || item.id}
				</p>
			</button>
		{:else}
			<p class="truncate text-body-md text-fg/85 px-1" title={item.title || item.id}>
				{item.title || item.id}
			</p>
		{/if}
	</div>
</article>
