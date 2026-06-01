<script lang="ts">
	import type { Snippet } from 'svelte';
	import Button from '../Button.svelte';

	export type ConfigFieldOption = { value: string; label: string };
	export type ConfigFieldMode = 'select' | 'action';
	export type ConfigFieldLayout = 'vertical' | 'horizontal';

	interface Props {
		label: string;
		mode?: ConfigFieldMode;
		layout?: ConfigFieldLayout;
		value?: string;
		id?: string;
		options?: ConfigFieldOption[];
		/** First option with value="" — set disabled:true for a non-selectable placeholder. */
		emptyOption?: { label: string; disabled?: boolean };
		buttonLabel?: string;
		placeholder?: string;
		disabled?: boolean;
		/** Replaces the visible label with custom content (e.g. a Chip). The label text is still used for a11y. */
		labelContent?: Snippet;
		onchange?: (value: string) => void;
		onButtonClick?: () => void;
	}

	let {
		label,
		mode = 'select',
		layout = 'vertical',
		value = $bindable(''),
		id,
		options = [],
		emptyOption,
		buttonLabel = 'Change',
		placeholder = '—',
		disabled = false,
		labelContent,
		onchange,
		onButtonClick,
	}: Props = $props();

	const fieldId = $derived(id ?? `field-${label.toLowerCase().replace(/\s+/g, '-')}`);

	const labelClass = 'font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase';
	const selectClass =
		'h-10 cursor-pointer rounded-md border border-rim bg-panel py-2 pr-8 pl-2 text-body-md text-fg disabled:cursor-not-allowed disabled:opacity-40';

	function handleChange(e: Event) {
		const v = (e.currentTarget as HTMLSelectElement).value;
		value = v;
		onchange?.(v);
	}
</script>

{#if layout === 'horizontal'}
	<div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
		<div class="flex min-w-0 items-center gap-2">
			{#if labelContent}
				<label class="sr-only" for={fieldId}>{label}</label>
				{@render labelContent()}
			{:else}
				<label class={labelClass} for={fieldId}>{label}</label>
			{/if}
		</div>
		{#if mode === 'select'}
			<select
				id={fieldId}
				class="{selectClass} min-w-40 max-w-56 truncate"
				{value}
				{disabled}
				onchange={handleChange}
			>
				{#if emptyOption}
					<option value="" disabled={emptyOption.disabled}>{emptyOption.label}</option>
				{/if}
				{#each options as opt (opt.value)}
					<option value={opt.value}>{opt.label}</option>
				{/each}
			</select>
		{/if}
	</div>
{:else}
	<div class="flex flex-col gap-1.5 text-left">
		{#if labelContent}
			<label class="sr-only" for={fieldId}>{label}</label>
			{@render labelContent()}
		{:else}
			<label class={labelClass} for={fieldId}>{label}</label>
		{/if}
		{#if mode === 'select'}
			<select
				id={fieldId}
				class={selectClass}
				bind:value
				{disabled}
				onchange={(e) => onchange?.((e.currentTarget as HTMLSelectElement).value)}
			>
				{#if emptyOption}
					<option value="" disabled={emptyOption.disabled}>{emptyOption.label}</option>
				{/if}
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
{/if}
