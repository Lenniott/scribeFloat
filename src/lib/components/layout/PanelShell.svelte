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

<div
	class="shadow-ambient flex h-[min(720px,90dvh)] w-full max-w-5xl flex-col overflow-hidden rounded-md bg-surface-low {className}"
>
	{@render children?.()}
</div>
