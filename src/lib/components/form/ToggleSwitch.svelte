<script lang="ts">
	let {
		checked = $bindable(false),
		disabled = false,
		id,
		label,
		labelFirst = false,
		class: className = "",
		"aria-label": ariaLabel,
		onchange,
	}: {
		checked?: boolean;
		disabled?: boolean;
		id?: string;
		label?: string;
		labelFirst?: boolean;
		class?: string;
		"aria-label"?: string;
		onchange?: (next: boolean) => void;
	} = $props();

	const switchId = $derived(
		id ?? (label ? `toggle-${label.toLowerCase().replace(/\s+/g, "-")}` : undefined),
	);
	const labelId = $derived(switchId ? `${switchId}-label` : undefined);

	const labelClass = $derived(
		`flex items-center gap-2 ${disabled ? "cursor-not-allowed opacity-40" : "cursor-pointer"} ${className}`.trim(),
	);
</script>

{#if label}
	<label for={switchId} class={labelClass}>
		{#if labelFirst}
			<span id={labelId} class="sf-field-label">{label}</span>
		{/if}
		<button
			type="button"
			role="switch"
			id={switchId}
			aria-checked={checked}
			aria-labelledby={labelId}
			class="relative inline-flex h-6 w-10 shrink-0 items-center rounded-full border transition-colors disabled:opacity-40 {checked
				? 'border-active bg-active'
				: 'border-rim bg-card'}"
			{disabled}
			onclick={() => {
				if (!disabled) {
					checked = !checked;
					onchange?.(checked);
				}
			}}
		>
			<span
				aria-hidden="true"
				class="absolute left-1 h-3.5 w-3.5 rounded-full transition-transform {checked
					? 'translate-x-4 bg-on-brand'
					: 'translate-x-0 bg-fg'}"
			></span>
		</button>
		{#if !labelFirst}
			<span id={labelId} class="sf-field-label">{label}</span>
		{/if}
	</label>
{:else}
	<button
		type="button"
		role="switch"
		{id}
		aria-checked={checked}
		aria-label={ariaLabel}
		class="relative inline-flex h-6 w-10 shrink-0 items-center rounded-full border transition-colors disabled:opacity-40 {checked
			? 'border-active bg-active'
			: 'border-rim bg-panel'}"
		{disabled}
		onclick={() => {
			if (!disabled) {
				checked = !checked;
				onchange?.(checked);
			}
		}}
	>
		<span
			aria-hidden="true"
			class="absolute left-1 h-3.5 w-3.5 rounded-full transition-transform {checked
				? 'translate-x-4 bg-on-brand'
				: 'translate-x-0 bg-fg'}"
		></span>
	</button>
{/if}
