<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		label,
		value = $bindable(""),
		id,
		placeholder = "",
		disabled = false,
		multiline = false,
		labelHidden = false,
		description,
		suffix,
		onblur,
	}: {
		label: string;
		value?: string;
		id?: string;
		placeholder?: string;
		disabled?: boolean;
		multiline?: boolean;
		labelHidden?: boolean;
		description?: string;
		/** Trailing control beside a single-line input (e.g. PathPicker Change button). */
		suffix?: Snippet;
		onblur?: () => void;
	} = $props();

	const fieldId = $derived(id ?? `field-${label.toLowerCase().replace(/\s+/g, "-")}`);

	const inputClass =
		"sf-input h-10 p-2 disabled:opacity-40 placeholder:text-fg-muted";
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class={labelHidden ? "sr-only" : "sf-field-label"} for={fieldId}>{label}</label>
	{#if description}
		<p class="sf-label-sm text-fg-muted">{description}</p>
	{/if}
	{#if multiline}
		<textarea
			id={fieldId}
			bind:value
			{placeholder}
			{disabled}
			onblur={onblur}
			class="sf-input min-h-[80px] resize-y p-2 disabled:opacity-40 placeholder:text-fg-muted"
		></textarea>
	{:else if suffix}
		<div class="flex min-w-0 items-center gap-2">
			<input
				id={fieldId}
				type="text"
				bind:value
				{placeholder}
				{disabled}
				class="{inputClass} min-w-0 flex-1"
				onblur={onblur}
			/>
			{@render suffix()}
		</div>
	{:else}
		<input
			id={fieldId}
			type="text"
			bind:value
			{placeholder}
			{disabled}
			class={inputClass}
			onblur={onblur}
		/>
	{/if}
</div>
