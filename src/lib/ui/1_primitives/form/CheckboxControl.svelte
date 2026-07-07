<script lang="ts">
	let {
		checked = $bindable(false),
		disabled = false,
		id,
		name,
		value = "on",
		"aria-label": ariaLabel,
		onchange,
	}: {
		checked?: boolean;
		disabled?: boolean;
		id?: string;
		name?: string;
		value?: string;
		"aria-label"?: string;
		onchange?: (next: boolean) => void;
	} = $props();
</script>

<input
	type="checkbox"
	{id}
	{name}
	{value}
	bind:checked
	{disabled}
	aria-label={ariaLabel}
	class="peer sr-only"
	onchange={(e) => onchange?.(e.currentTarget.checked)}
/>
<span aria-hidden="true" class="sf-checkbox-box">
	<span class="sf-label-sm leading-none">✓</span>
</span>
