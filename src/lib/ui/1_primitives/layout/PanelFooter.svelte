<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		class: className = '',
		children,
	}: {
		class?: string;
		children?: Snippet;
	} = $props();
</script>

<!--
  Pane footer: shrink-0 flex sibling below a flex-1 scroll region.
  Not position:fixed/sticky — content scrolls in the middle; actions stay at the bottom of the pane.
-->
<footer
	class="flex min-h-[3.25rem] shrink-0 items-center justify-end gap-1 border-t border-card/60 bg-panel px-4 py-3 {className}"
>
	{@render children?.()}
</footer>
