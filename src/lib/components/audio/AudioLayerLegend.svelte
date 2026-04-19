<script lang="ts">
	let {
		speakerEnabled = false,
	}: {
		speakerEnabled?: boolean;
	} = $props();
</script>

<div
	class="flex flex-wrap items-center justify-center gap-4 text-label-sm font-semibold text-on-surface"
	aria-label="Layer legend"
>
	<div class="flex items-center gap-2">
		<span class="h-2.5 w-2.5 shrink-0 rounded-sm bg-primary" aria-hidden="true"></span>
		<span>Mic</span>
	</div>
	{#if speakerEnabled}
	<div class="flex items-center gap-2">
		<span class="h-2.5 w-2.5 shrink-0 rounded-sm bg-tertiary" aria-hidden="true"></span>
		<span>Speaker</span>
	</div>
	{/if}
</div>
