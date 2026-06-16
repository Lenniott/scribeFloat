<script lang="ts">
	let {
		speakerEnabled = false,
	}: {
		speakerEnabled?: boolean;
	} = $props();
</script>

<div
	class="flex flex-wrap items-center justify-center gap-4 sf-label-sm text-fg"
	aria-label="Layer legend"
>
	<div class="flex items-center gap-2">
		<span class="h-2.5 w-2.5 shrink-0 rounded-sm bg-fg-muted" aria-hidden="true"></span>
		<span>Mic</span>
	</div>
	{#if speakerEnabled}
	<div class="flex items-center gap-2">
		<span class="h-2.5 w-2.5 shrink-0 rounded-sm bg-fg-dim" aria-hidden="true"></span>
		<span>Speaker</span>
	</div>
	{/if}
</div>
