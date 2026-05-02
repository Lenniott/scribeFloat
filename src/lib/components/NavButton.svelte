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
				? "bg-active/15 text-fg before:absolute before:top-0 before:left-0 before:h-full before:w-1 before:bg-active"
				: "text-fg/60 hover:bg-fill hover:text-fg",
		].join(" "),
	);
</script>

<button type="button" class={classes} aria-current={active ? "page" : undefined} onclick={onclick}>
	{@render children?.()}
</button>
