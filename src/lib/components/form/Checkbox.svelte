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
	class={`inline-flex items-center gap-2 text-on-surface ${disabled ? "cursor-not-allowed opacity-45" : "cursor-pointer"} ${className}`.trim()}
>
	<input
		type="checkbox"
		{id}
		{name}
		{value}
		bind:checked
		{disabled}
		aria-label={ariaLabel}
		class="peer sr-only"
	/>
	<span
		aria-hidden="true"
		class="relative inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-white/85 bg-transparent text-transparent transition-colors peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-primary peer-checked:border-primary peer-checked:bg-primary peer-checked:text-on-primary"
	>
		<span class="text-label-md leading-none font-bold">✓</span>
	</span>
	{#if label}
		<span class="text-body-md tracking-[0.01em]">{label}</span>
	{/if}
</label>
