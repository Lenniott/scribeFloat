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

<div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-1 {className}">
	{@render children?.()}
</div>
