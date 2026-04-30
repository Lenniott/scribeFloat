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
	<legend class="font-mono text-label-sm mb-1.5 font-normal tracking-stamped text-on-surface/80 uppercase">
		{label}
	</legend>
	<div
		class="inline-flex max-w-full rounded-md gap-1"
		role="radiogroup"
		aria-label={label}
	>
		{#each options as opt (opt.value)}
			<label
				class="flex cursor-pointer items-center justify-center rounded-sm px-3 py-1.5 text-label-md font-normal transition-colors {selected ===
				opt.value
					? 'bg-surface-highest text-on-surface'
					: 'text-on-surface hover:bg-surface-high'}"
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
