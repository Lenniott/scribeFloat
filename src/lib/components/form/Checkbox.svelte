<script lang="ts">
	let {
		checked = $bindable(false),
		disabled = false,
		id,
		label,
		name,
		value = "on",
		"aria-label": ariaLabel,
		class: className = "",
	}: {
		checked?: boolean;
		disabled?: boolean;
		id?: string;
		label?: string;
		name?: string;
		value?: string;
		"aria-label"?: string;
		class?: string;
	} = $props();
</script>

<label
	for={id}
	class={`inline-flex items-center gap-2 text-fg ${disabled ? "cursor-not-allowed opacity-45" : "cursor-pointer"} ${className}`.trim()}
>
	<input
		type="checkbox"
		{id}
		{name}
		{value}
		bind:checked
		{disabled}
		aria-label={ariaLabel}
		class="peer sr-only focus:ring-0"
	/>
	<span
		aria-hidden="true"
		class="relative inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-rim bg-panel text-transparent transition-colors peer-checked:border-brand peer-checked:bg-brand peer-checked:text-on-brand peer-focus:ring-2 peer-focus:ring-warning peer-focus:ring-offset-2 peer-focus:ring-offset-panel peer-focus:rounded-sm"
	>
		<span class="sf-label-sm leading-none">✓</span>
	</span>
	{#if label}
		<span class="sf-body-md">{label}</span>
	{/if}
</label>
