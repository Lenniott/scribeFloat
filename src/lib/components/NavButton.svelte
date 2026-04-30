<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		active = false,
		children,
		onclick,
	}: {
		active?: boolean;
		children?: Snippet;
		onclick?: (event: MouseEvent) => void;
	} = $props();

	const base =
		"relative flex w-full cursor-pointer items-center px-4 py-2 text-left text-sm font-normal transition-colors";

	let classes = $derived(
		[
			base,
			active
				? "bg-surface-highest text-on-surface before:absolute before:top-0 before:left-0 before:h-full before:w-1 before:bg-on-surface/40"
				: "text-on-surface/60 hover:bg-surface-high hover:text-on-surface",
		].join(" "),
	);
</script>

<button type="button" class={classes} aria-current={active ? "page" : undefined} onclick={onclick}>
	{@render children?.()}
</button>
