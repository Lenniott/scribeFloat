<script lang="ts">
	let {
		checked = $bindable(false),
		disabled = false,
		id,
		"aria-label": ariaLabel,
	}: {
		checked?: boolean;
		disabled?: boolean;
		id?: string;
		"aria-label"?: string;
	} = $props();
</script>

<button
	type="button"
	role="switch"
	{id}
	aria-checked={checked}
	aria-label={ariaLabel}
	class="relative h-6 w-10 shrink-0 rounded-md transition-colors disabled:opacity-40 {checked
		? 'bg-primary'
		: 'bg-surface-variant'}"
	{disabled}
	onclick={() => {
		if (!disabled) checked = !checked;
	}}
>
	<span
		class="absolute top-1 left-1 h-4 w-4 rounded-sm bg-on-surface transition-transform {checked
			? 'translate-x-4'
			: 'translate-x-0'}"
	></span>
</button>
