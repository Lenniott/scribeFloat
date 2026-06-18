<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		class: className = "",
		children,
	}: {
		class?: string;
		children?: Snippet;
	} = $props();
</script>

<div class={`divide-y divide-fill overflow-hidden rounded-md border border-fill bg-panel ${className}`.trim()}>
	{#if children}
		{@render children()}
	{/if}
</div>
