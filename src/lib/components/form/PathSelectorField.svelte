<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import Button from "../Button.svelte";

	let {
		label,
		path = $bindable(""),
		directory = true,
		onChange,
	}: {
		label: string;
		path?: string;
		directory?: boolean;
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
	<label class="font-mono text-label-sm font-normal tracking-stamped text-on-surface/80 uppercase" for={fieldId}
		>{label}</label
	>
	<div class="flex min-w-0 items-center gap-2">
		<input
			id={fieldId}
			type="text"
			bind:value={path}
			class="h-8 min-w-0 flex-1 rounded-md border-0 border-b border-transparent bg-surface-lowest p-2 text-body-md text-on-surface placeholder:text-on-surface-dim focus:bg-surface-highest focus:outline-none"
			onblur={() => onChange?.(path)}
		/>
		<Button variant="normal" onclick={choosePath}>Change</Button>
	</div>
</div>
