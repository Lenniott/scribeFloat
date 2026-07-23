<script lang="ts">
	import CheckboxControl from '@primitives/form/CheckboxControl.svelte';

	let {
		checked = $bindable(false),
		label,
		count,
		onchange,
	}: {
		checked?: boolean;
		label: string;
		count: number;
		onchange?: (next: boolean) => void;
	} = $props();
</script>

<label class="flex cursor-pointer items-center gap-2.5 rounded-md px-1 py-1 hover:bg-fill">
	<CheckboxControl bind:checked {onchange} aria-label={label} />
	<span class="min-w-0 flex-1 truncate sf-label-md text-fg">{label}</span>
	<span class="sf-meta-sm shrink-0 text-fg-dim">{count}</span>
</label>
