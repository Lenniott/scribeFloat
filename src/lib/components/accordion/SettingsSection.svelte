<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		title,
		children,
	}: {
		title?: string;
		children?: Snippet;
	} = $props();
</script>

<section class="flex flex-col gap-4 pt-1">
	{#if title}
		<h4 class="sf-label-sm text-fg-dim">{title}</h4>
	{/if}
	<div class="flex flex-col gap-4">
		{@render children?.()}
	</div>
</section>
