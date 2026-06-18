<script lang="ts">
	import type { Component } from 'svelte';
	import Chip from '@lib/components/Chip.svelte';

	let {
		label,
		icon: Icon,
		active = false,
		disabled = false,
		accent = false,
		badge,
		onclick,
	}: {
		label: string;
		icon: Component;
		active?: boolean;
		disabled?: boolean;
		accent?: boolean;
		badge?: string;
		onclick?: () => void;
	} = $props();

	const base =
		'flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left sf-label-md transition-colors';

	let classes = $derived(
		[
			base,
			disabled
				? 'cursor-not-allowed text-fg-muted opacity-60'
				: active
					? 'bg-fill text-fg'
					: accent
						? 'text-brand hover:bg-fill hover:text-brand'
						: 'text-fg-dim hover:bg-fill hover:text-fg',
		].join(' '),
	);
</script>

<button
	type="button"
	class={classes}
	aria-current={active ? 'page' : undefined}
	aria-disabled={disabled || undefined}
	disabled={disabled}
	{onclick}
>
	<Icon class="size-3.5 shrink-0" aria-hidden="true" />
	<span class="min-w-0 flex-1 truncate">{label}</span>
	{#if badge}
		<Chip variant="muted">{badge}</Chip>
	{/if}
</button>
