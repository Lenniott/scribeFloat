<script lang="ts">
	import Button from "../Button.svelte";

	export type ConfigFieldOption = { value: string; label: string };
	export type ConfigFieldMode = "select" | "action";

	let {
		label,
		mode = "select",
		value = $bindable(""),
		id,
		options = [],
		buttonLabel = "Change",
		placeholder = "—",
		onButtonClick,
	}: {
		label: string;
		mode?: ConfigFieldMode;
		value?: string;
		id?: string;
		options?: ConfigFieldOption[];
		buttonLabel?: string;
		placeholder?: string;
		onButtonClick?: () => void;
	} = $props();

	const fieldId = $derived(id ?? `field-${label.toLowerCase().replace(/\s+/g, "-")}`);
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase" for={fieldId}
		>{label}</label
	>
	{#if mode === "select"}
		<select
			id={fieldId}
			bind:value
			class="h-10 flex items-center rounded-md border-0 border-b border-transparent bg-panel py-2 pr-8 pl-2 text-body-md text-fg"
		>
			{#each options as opt (opt.value)}
				<option value={opt.value}>{opt.label}</option>
			{/each}
		</select>
	{:else}
		<div class="flex min-w-0 items-center gap-2">
			<code
				id={fieldId}
				class="h-10 flex items-center font-mono text-label-md min-w-0 flex-1 truncate rounded-md bg-panel px-2 py-2 tracking-stamped text-fg/90"
				title={value}
			>
				{value || placeholder}
			</code>
			<Button variant="normal" onclick={() => onButtonClick?.()}>{buttonLabel}</Button>
		</div>
	{/if}
</div>
