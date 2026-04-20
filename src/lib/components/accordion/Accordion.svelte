<script lang="ts">
	import { setContext } from "svelte";
	import type { Snippet } from "svelte";
	import { ACCORDION_KEY, type AccordionContextState } from "./accordion-context.js";

	let {
		children,
		defaultOpenId = null,
	}: {
		children?: Snippet;
		defaultOpenId?: string | null;
	} = $props();

	// svelte-ignore state_referenced_locally
	const initialOpenId = defaultOpenId;

	const ctx = $state<AccordionContextState>({
		openId: initialOpenId,
		toggle(id: string) {
			ctx.openId = ctx.openId === id ? null : id;
		},
	});

	setContext(ACCORDION_KEY, ctx);
</script>

<div class="flex min-h-0 flex-col pt-3">
	{@render children?.()}
</div>
