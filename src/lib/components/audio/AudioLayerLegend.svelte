<script lang="ts">
	let {
		speakerEnabled = false,
	}: {
		speakerEnabled?: boolean;
	} = $props();
</script>

<div
	class="font-mono flex flex-wrap items-center justify-center gap-4 text-label-sm font-normal tracking-stamped text-fg"
	aria-label="Layer legend"
>
	<div class="flex items-center gap-2">
		<span class="h-2.5 w-2.5 shrink-0 rounded-sm bg-fg-muted" aria-hidden="true"></span>
		<span>Mic</span>
	</div>
	{#if speakerEnabled}
	<div class="flex items-center gap-2">
		<span class="h-2.5 w-2.5 shrink-0 rounded-sm bg-fg/40" aria-hidden="true"></span>
		<span>Speaker</span>
	</div>
	{/if}
</div>
