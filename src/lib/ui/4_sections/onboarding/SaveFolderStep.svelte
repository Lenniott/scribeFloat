<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@components/controls/Button.svelte";
	import PathPicker from "@components/controls/PathPicker.svelte";
	import StepShell from "@primitives/layout/StepFrame.svelte";
	import { appErrorMessage } from '@utils/types';

	let {
		onBack,
		onNext,
	}: {
		onBack: () => void;
		onNext: () => void;
	} = $props();

	let outputPath = $state("");
	let error = $state("");

	async function saveOutputPath(path: string) {
		error = "";
		try {
			await invoke("settings_set_output_path", { path });
		} catch (e) {
			error = `Could not save folder: ${appErrorMessage(e)}`;
			outputPath = await invoke<string>("settings_get_output_path").catch(() => outputPath);
		}
	}

	onMount(async () => {
		outputPath = await invoke<string>("settings_get_output_path").catch(() => "");
	});
</script>

<StepShell
	title="Where should ScribeFloat save your notes?"
	subtitle="Notes and transcripts are saved here as plain markdown files. You can change this later in Settings."
>
	{#snippet children()}
		<div class="space-y-2">
			<PathPicker
				label="Save folder"
				bind:path={outputPath}
				onChange={(next) => void saveOutputPath(next)}
			/>
			{#if error}
				<p class="sf-label-sm text-destructive px-1">{error}</p>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={onNext}>Continue</Button>
	{/snippet}
</StepShell>
