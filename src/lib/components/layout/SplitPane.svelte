<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		class: className = "",
		left,
		right,
	}: {
		class?: string;
		left?: Snippet;
		right?: Snippet;
	} = $props();
</script>

<div
	class="grid min-h-0 flex-1 grid-cols-1 gap-0 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] {className}"
>
	<div class="flex min-h-0 min-w-0 flex-col bg-surface-low">
		{@render left?.()}
	</div>
	<div class="flex min-h-0 min-w-0 flex-col bg-surface-lowest">
		{@render right?.()}
	</div>
</div>
