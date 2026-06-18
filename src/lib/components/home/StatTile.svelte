<script lang="ts">
	let {
		value,
		label,
		highlight = false,
	}: {
		value: string;
		label: string;
		highlight?: boolean;
	} = $props();
</script>

<div class="rounded-md border border-fill bg-card p-4">
	<p
		class="sf-headline-sm tabular-nums {highlight ? 'text-warning' : 'text-fg'}"
	>
		{value}
	</p>
	<p class="sf-label-sm mt-1 text-fg-dim">{label}</p>
</div>
