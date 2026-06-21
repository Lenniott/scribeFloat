<script lang="ts">
	export type Option = { value: string; label: string };

	let {
		label,
		options,
		selected = $bindable(""),
		name,
		labelHidden = false,
		description,
	}: {
		label: string;
		options: Option[];
		selected?: string;
		name: string;
		labelHidden?: boolean;
		description?: string;
	} = $props();
</script>

<fieldset class="flex flex-col gap-2 text-left">
	<legend class={labelHidden ? "sr-only" : "sf-field-label mb-1.5"}>{label}</legend>
	{#if description}
		<p class="sf-label-sm text-fg-muted">{description}</p>
	{/if}
	<div
		class="inline-flex max-w-full rounded-md gap-1"
		role="radiogroup"
		aria-label={label}
	>
		{#each options as opt (opt.value)}
			<label
				class="flex cursor-pointer items-center justify-center rounded-sm px-3 py-1.5 sf-label-md transition-colors sf-focus-ring {selected ===
				opt.value
					? 'bg-active text-on-active'
					: 'text-fg hover:bg-panel'}"
			>
				<input
					type="radio"
					class="sr-only"
					{name}
					value={opt.value}
					checked={selected === opt.value}
					onchange={() => { selected = opt.value; }}
				/>
				{opt.label}
			</label>
		{/each}
	</div>
</fieldset>
