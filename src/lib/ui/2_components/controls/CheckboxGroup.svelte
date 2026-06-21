<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		label,
		labelHidden = false,
		class: className = '',
		children,
	}: {
		label?: string;
		labelHidden?: boolean;
		class?: string;
		children?: Snippet;
	} = $props();
</script>

<fieldset class={`flex flex-col gap-1 border-0 p-0 ${className}`.trim()}>
	{#if label}
		<legend class={labelHidden ? 'sr-only' : 'sf-section-label mb-2 text-fg-dim'}>{label}</legend>
	{/if}
	{@render children?.()}
</fieldset>
