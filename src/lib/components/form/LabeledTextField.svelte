<script lang="ts">
	let {
		label,
		value = $bindable(""),
		id,
		placeholder = "",
		disabled = false,
	}: {
		label: string;
		value?: string;
		id?: string;
		placeholder?: string;
		disabled?: boolean;
	} = $props();

	const fieldId = $derived(id ?? `field-${label.toLowerCase().replace(/\s+/g, "-")}`);
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase" for={fieldId}
		>{label}</label
	>
	<input
		id={fieldId}
		type="text"
		bind:value
		{placeholder}
		{disabled}
		class="rounded-md h-10 flex items-center border-0 border-b border-transparent bg-panel p-2 text-body-md text-fg placeholder:text-fg-dim disabled:opacity-40"
	/>
</div>
