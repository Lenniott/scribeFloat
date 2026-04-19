<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		class: className = "",
		left,
		center,
		right,
	}: {
		class?: string;
		left?: Snippet;
		center?: Snippet;
		right?: Snippet;
	} = $props();
</script>

<header
	class="flex shrink-0 items-center justify-between gap-4 bg-surface-container-lowest px-4 py-3 {className}"
>
	<div class="min-w-0 flex-1 text-left">{@render left?.()}</div>
	<div class="shrink-0">{@render center?.()}</div>
	<div class="flex shrink-0 items-center justify-end gap-2">{@render right?.()}</div>
</header>
