<script lang="ts">
	export type DeviceOption = { value: string; label: string };

	let {
		label,
		options,
		value = $bindable(""),
		id,
	}: {
		label: string;
		options: DeviceOption[];
		value?: string;
		id?: string;
	} = $props();

	const selectId = $derived(id ?? `device-${label.toLowerCase().replace(/\s+/g, "-")}`);
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class="text-label-sm font-semibold tracking-wide text-on-surface/80 uppercase" for={selectId}
		>{label}</label
	>
	<select
		id={selectId}
		bind:value
		class="rounded-md border-0 border-b border-transparent bg-surface-container-lowest py-2 pr-8 pl-2 text-body-md text-on-surface focus:border-primary focus:bg-surface-container-high focus:ring-0 focus:outline-none"
	>
		{#each options as opt (opt.value)}
			<option value={opt.value}>{opt.label}</option>
		{/each}
	</select>
</div>
