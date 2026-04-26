<script lang="ts">
	let {
		checked = $bindable(false),
		disabled = false,
		id,
		"aria-label": ariaLabel,
		onchange,
	}: {
		checked?: boolean;
		disabled?: boolean;
		id?: string;
		"aria-label"?: string;
		onchange?: (next: boolean) => void;
	} = $props();
</script>

<button
	type="button"
	role="switch"
	{id}
	aria-checked={checked}
	aria-label={ariaLabel}
	class="relative inline-flex h-6 w-10 shrink-0 items-center rounded-full transition-colors disabled:opacity-40 {checked
		? 'bg-secondary'
		: 'bg-surface-container-lowest'}"
	{disabled}
	onclick={() => {
		if (!disabled) {
			checked = !checked;
			onchange?.(checked);
		}
	}}
>
	<span
		class="absolute left-1 h-4 w-4 rounded-full transition-transform {checked
			? 'translate-x-4 bg-void'
			: 'translate-x-0 bg-surface-container-highest'}"
	></span>
</button>
