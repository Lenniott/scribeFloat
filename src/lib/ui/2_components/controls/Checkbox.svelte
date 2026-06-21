<script lang="ts">
	import CheckboxControl from '@primitives/form/CheckboxControl.svelte';

	let {
		checked = $bindable(false),
		disabled = false,
		id,
		label,
		name,
		value = "on",
		"aria-label": ariaLabel,
		class: className = "",
		onchange,
	}: {
		checked?: boolean;
		disabled?: boolean;
		id?: string;
		label?: string;
		name?: string;
		value?: string;
		"aria-label"?: string;
		class?: string;
		onchange?: (next: boolean) => void;
	} = $props();

	const fieldId = $derived(
		id ?? (label ? `checkbox-${label.toLowerCase().replace(/\s+/g, "-")}` : undefined),
	);
</script>

<label
	class={`inline-flex items-center gap-2 text-fg ${disabled ? "cursor-not-allowed opacity-45" : "cursor-pointer"} ${className}`.trim()}
>
	<CheckboxControl
		id={fieldId}
		bind:checked
		{disabled}
		{name}
		{value}
		aria-label={label ? undefined : ariaLabel}
		{onchange}
	/>
	{#if label}
		<span class="sf-body-md">{label}</span>
	{/if}
</label>
