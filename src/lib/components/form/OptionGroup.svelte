<script lang="ts">
	export type Option = { value: string; label: string };

	let {
		label,
		options,
		selected = $bindable(""),
		name,
	}: {
		label: string;
		options: Option[];
		selected?: string;
		name: string;
	} = $props();
</script>

<fieldset class="flex flex-col gap-2 text-left">
	<legend class="text-label-sm mb-1.5 font-semibold tracking-wide text-on-surface/80 uppercase">
		{label}
	</legend>
	<div
		class="inline-flex max-w-full rounded-md gap-1"
		role="radiogroup"
		aria-label={label}
	>
		{#each options as opt (opt.value)}
			<label
				class="flex cursor-pointer items-center justify-center rounded-sm px-3 py-1.5 text-label-md font-medium transition-colors {selected ===
				opt.value
					? 'bg-tertiary text-on-tertiary'
					: 'text-on-surface hover:bg-tertiary/20'}"
			>
				<input
					type="radio"
					class="sr-only h-7 bg-surface-container-low border border-primary"
					{name}
					value={opt.value}
					bind:group={selected}
				/>
				{opt.label}
			</label>
		{/each}
	</div>
</fieldset>
