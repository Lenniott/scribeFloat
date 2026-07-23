<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		title,
		description,
		class: className = "",
		children,
		action,
	}: {
		title: string;
		description?: string;
		class?: string;
		children?: Snippet;
		action?: Snippet;
	} = $props();
</script>

<section class={`flex flex-col gap-2 ${className}`.trim()}>
	<div class="flex items-start justify-between gap-3">
		<div class="flex min-w-0 flex-col gap-1">
			<h3 class="sf-section-label text-fg-dim">{title}</h3>
			{#if description}
				<p class="sf-label-sm text-fg-dim">{description}</p>
			{/if}
		</div>
		{#if action}
			<div class="shrink-0">
				{@render action()}
			</div>
		{/if}
	</div>
	{#if children}
		{@render children()}
	{/if}
</section>
