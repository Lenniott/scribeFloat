<script lang="ts">
	let {
		label,
		value = $bindable(""),
		id,
		placeholder = "",
		disabled = false,
		multiline = false,
	}: {
		label: string;
		value?: string;
		id?: string;
		placeholder?: string;
		disabled?: boolean;
		multiline?: boolean;
	} = $props();

	const fieldId = $derived(id ?? `field-${label.toLowerCase().replace(/\s+/g, "-")}`);
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class="sf-field-label" for={fieldId}>{label}</label>
	{#if multiline}
		<textarea
			id={fieldId}
			bind:value
			{placeholder}
			{disabled}
			class="sf-input min-h-[80px] resize-y p-2 disabled:opacity-40 placeholder:text-fg-muted"
		></textarea>
	{:else}
		<input
			id={fieldId}
			type="text"
			bind:value
			{placeholder}
			{disabled}
			class="sf-input h-10 p-2 disabled:opacity-40 placeholder:text-fg-muted"
		/>
	{/if}
</div>
