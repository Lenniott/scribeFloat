<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import Button from "../Button.svelte";

	let {
		label,
		path = $bindable(""),
		directory = true,
		labelHidden = false,
		description,
		onChange,
	}: {
		label: string;
		path?: string;
		directory?: boolean;
		labelHidden?: boolean;
		description?: string;
		onChange?: (nextPath: string) => void | Promise<void>;
	} = $props();
	const fieldId = $derived(`path-${label.toLowerCase().replace(/\s+/g, "-")}`);

	async function choosePath() {
		const selected = await open({
			directory,
			multiple: false,
			defaultPath: path || undefined,
		}).catch(() => null);
		if (typeof selected !== "string" || !selected) return;
		path = selected;
		await onChange?.(selected);
	}
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class={labelHidden ? "sr-only" : "sf-field-label"} for={fieldId}>{label}</label>
	{#if description}
		<p class="sf-label-sm text-fg-muted">{description}</p>
	{/if}
	<div class="flex min-w-0 items-center gap-2">
		<input
			id={fieldId}
			type="text"
			bind:value={path}
			class="sf-input h-10 min-w-0 flex-1 p-2 placeholder:text-fg-muted"
			onblur={() => onChange?.(path)}
		/>
		<Button variant="normal" onclick={choosePath}>Change</Button>
	</div>
</div>
