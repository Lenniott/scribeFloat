<script lang="ts">
	import type { Snippet } from "svelte";
	import StepProgress from "./StepProgress.svelte";

	let {
		currentStep,
		title,
		subtitle = "",
		children,
		footer,
	}: {
		currentStep: number;
		title: string;
		subtitle?: string;
		children?: Snippet;
		footer?: Snippet;
	} = $props();
</script>

<div class="flex flex-col w-full h-full">
	<div class="flex flex-col gap-1 mb-5">
		<StepProgress {currentStep} />
		<h1 class="sf-headline-sm text-fg mt-2">{title}</h1>
		{#if subtitle}
			<p class="text-body-md text-fg-dim">{subtitle}</p>
		{/if}
	</div>

	<div class="flex-1 min-h-0 overflow-y-auto">
		{@render children?.()}
	</div>

	{#if footer}
		<footer class="pt-4 flex items-center justify-between gap-3 shrink-0">
			{@render footer()}
		</footer>
	{/if}
</div>
