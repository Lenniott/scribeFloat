<script lang="ts">
	import { getContext } from "svelte";
	import type { Snippet } from "svelte";
	import { ACCORDION_KEY, type AccordionContextState } from "./accordion-context.js";

	let {
		id,
		title,
		children,
	}: {
		id: string;
		title: string;
		children?: Snippet;
	} = $props();

	const ctx = getContext<AccordionContextState>(ACCORDION_KEY);
	if (!ctx) {
		throw new Error("AccordionItem must be used inside Accordion");
	}

	const isOpen = $derived(ctx.openId === id);
</script>

<div class="bg-card mb-1">
	<h3 class="border-0">
		<button
			type="button"
			class="font-mono text-label-md flex w-full items-center justify-between px-3 py-2.5 text-left font-normal tracking-stamped text-fg uppercase hover:bg-fillest"
			aria-expanded={isOpen}
			aria-controls={`panel-${id}`}
			id={`header-${id}`}
			onclick={() => ctx.toggle(id)}
		>
			{title}
			<span class="text-fg/50" aria-hidden="true">{isOpen ? "−" : "+"}</span>
		</button>
	</h3>
	<div
		id={`panel-${id}`}
		role="region"
		aria-labelledby={`header-${id}`}
		class="grid transition-[grid-template-rows] duration-200 ease-out {isOpen
			? 'grid-rows-[1fr]'
			: 'grid-rows-[0fr]'}"
	>
		<div class="min-h-0 overflow-hidden bg-fill">
			<div class="p-3">
				{@render children?.()}
			</div>
		</div>
	</div>
</div>
