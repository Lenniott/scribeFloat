<script lang="ts">
	import type { Snippet } from "svelte";

	let { children }: { children?: Snippet } = $props();
</script>

<footer
	class="flex shrink-0 items-center justify-end gap-3 border-t border-outline-variant/0 bg-surface-container-low px-4 py-3"
	style="box-shadow: inset 0 1px 0 0 var(--color-outline-variant);"
>
	{@render children?.()}
</footer>
