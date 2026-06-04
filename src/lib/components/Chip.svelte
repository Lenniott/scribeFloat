<script lang="ts">
	export type ChipVariant = 'brand' | 'focus' | 'muted';

	let {
		variant,
		children,
	}: {
		variant: ChipVariant;
		children?: import('svelte').Snippet;
	} = $props();

	const variantClass: Record<ChipVariant, string> = {
		brand: 'border-brand bg-brand/10 text-brand',
		focus: 'border-focus bg-focus/15 text-focus',
		muted: 'border-card bg-fill text-fg/55',
	};
</script>

<span
	class="sf-label-sm shrink-0 rounded-sm border px-1.5 py-0.5 text-center {variantClass[variant]}"
>
	{@render children?.()}
</span>
