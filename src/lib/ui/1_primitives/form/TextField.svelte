<script lang="ts">
	let {
		label,
		value = $bindable(""),
		id,
		placeholder = "",
		disabled = false,
		multiline = false,
		labelHidden = false,
		description,
		onblur,
	}: {
		label: string;
		value?: string;
		id?: string;
		placeholder?: string;
		disabled?: boolean;
		multiline?: boolean;
		labelHidden?: boolean;
		description?: string;
		onblur?: (event: FocusEvent) => void;
	} = $props();

	const fieldId = $derived(id ?? `field-${label.toLowerCase().replace(/\s+/g, "-")}`);
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class={labelHidden ? "sr-only" : "sf-field-label"} for={fieldId}>{label}</label>
	{#if description}
		<p class="sf-label-sm text-fg-muted">{description}</p>
	{/if}
	{#if multiline}
		<textarea
			id={fieldId}
			bind:value
			{placeholder}
			{disabled}
			onblur={onblur}
			class="sf-input min-h-[80px] resize-y p-2 disabled:opacity-40 placeholder:text-fg-muted"
		></textarea>
	{:else}
		<input
			id={fieldId}
			type="text"
			bind:value
			{placeholder}
			{disabled}
			onblur={onblur}
			class="sf-input h-10 p-2 disabled:opacity-40 placeholder:text-fg-muted"
		/>
	{/if}
</div>
