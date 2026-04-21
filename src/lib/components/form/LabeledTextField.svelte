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
	<label class="text-label-sm font-semibold tracking-wide text-on-surface/80 uppercase" for={fieldId}
		>{label}</label
	>
	<input
		id={fieldId}
		type="text"
		bind:value
		{placeholder}
		{disabled}
		class="rounded-md border-0 border-b border-transparent bg-sur px-2 py-2 text-body-md text-on-surface placeholder:text-on-surface/40 focus:ring-tertiary focus:bg-surface-container-highest focus:outline-none disabled:opacity-40"
	/>
</div>
