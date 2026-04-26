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
	<label class="font-data text-label-sm font-normal tracking-widest text-on-surface/80 uppercase" for={fieldId}
		>{label}</label
	>
	<input
		id={fieldId}
		type="text"
		bind:value
		{placeholder}
		{disabled}
		class="rounded-md h-8 border-0 border-b border-transparent bg-surface-container-lowest p-2 text-body-md text-on-surface placeholder:text-on-surface-dim focus:ring-active focus:bg-surface-container-highest focus:outline-none disabled:opacity-40"
	/>
</div>
