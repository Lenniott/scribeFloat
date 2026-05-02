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
	<legend class="font-mono text-label-sm mb-1.5 font-normal tracking-stamped text-fg/80 uppercase">
		{label}
	</legend>
	<div
		class="inline-flex max-w-full rounded-md gap-1"
		role="radiogroup"
		aria-label={label}
	>
		{#each options as opt (opt.value)}
			<label
				class="flex cursor-pointer items-center justify-center rounded-sm px-3 py-1.5 text-label-md font-normal transition-colors outline-none focus-within:ring-2 focus-within:ring-focus focus-within:ring-offset-2 focus-within:ring-offset-canvas {selected ===
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
