<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import ConfigField from "@lib/components/form/ConfigField.svelte";
	import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
	import PathSelectorField from "@lib/components/form/PathSelectorField.svelte";

	let outputPath = $state("");
	let openHotkey = $state("");
	let dictateHotkey = $state("");
	let inputLabel = $state("");
	let outputLabel = $state("");
	let message = $state("");

	async function refresh() {
		outputPath = await invoke<string>("settings_get_output_path").catch(() => "");
		const [open, dictate] = await invoke<[string, string]>("settings_get_hotkeys").catch(() => ["", ""]);
		openHotkey = open;
		dictateHotkey = dictate;
		const [inLabel, outLabel] = await invoke<[string, string]>("settings_get_input_labels").catch(() => [
			"Mic",
			"Speaker",
		]);
		inputLabel = inLabel;
		outputLabel = outLabel;
	}

	async function saveAll() {
		message = "";
		await invoke("settings_set_output_path", { path: outputPath });
		await invoke("settings_set_hotkeys", { openScribe: openHotkey, dictate: dictateHotkey });
		await invoke("settings_set_input_labels", { inputLabel, outputLabel });
		message = "Saved";
	}

	onMount(refresh);
</script>

<section class="space-y-4">
	<h2 class="text-title-sm font-semibold">General settings</h2>
	<PathSelectorField label="Default save folder" bind:path={outputPath} />
	<ConfigField label="Open Scribe hotkey" mode="action" bind:value={openHotkey} />
	<ConfigField label="Dictate hotkey" mode="action" bind:value={dictateHotkey} />
	<LabeledTextField label="Input label" bind:value={inputLabel} />
	<LabeledTextField label="Output label" bind:value={outputLabel} />
	<div class="flex items-center gap-3">
		<Button variant="primary" onclick={saveAll}>Save</Button>
		{#if message}
			<p class="text-label-sm text-on-surface/70">{message}</p>
		{/if}
	</div>
</section>