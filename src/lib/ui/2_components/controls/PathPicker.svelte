<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import TextField from "@primitives/form/TextField.svelte";
	import Button from "./Button.svelte";

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

<TextField
	{label}
	{labelHidden}
	{description}
	bind:value={path}
	onblur={() => onChange?.(path)}
>
	{#snippet suffix()}
		<Button variant="normal" onclick={choosePath}>Change</Button>
	{/snippet}
</TextField>
