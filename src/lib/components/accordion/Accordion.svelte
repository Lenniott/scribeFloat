<script lang="ts">
	import { setContext } from "svelte";
	import type { Snippet } from "svelte";
	import { ACCORDION_KEY, type AccordionContextState } from "./accordion-context.js";

	let { children }: { children?: Snippet } = $props();

	const ctx = $state<AccordionContextState>({
		openId: null,
		toggle(id: string) {
			ctx.openId = ctx.openId === id ? null : id;
		},
	});

	setContext(ACCORDION_KEY, ctx);
</script>

<div class="flex min-h-0 flex-col">
	{@render children?.()}
</div>
