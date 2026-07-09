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

<div class={`overflow-hidden ${className}`.trim()}>
	{#if children}
		{@render children()}
	{/if}
</div>
