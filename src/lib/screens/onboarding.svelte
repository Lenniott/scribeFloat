<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import ConfigField from "@lib/components/form/ConfigField.svelte";
	import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
	import PathSelectorField from "@lib/components/form/PathSelectorField.svelte";
	import ModelSetupModal, { type ModelListItem } from "@lib/components/model/ModelSetupModal.svelte";

	type PermissionStatus = {
		kind: string;
		granted: boolean;
		can_request: boolean;
	};

	let { onComplete }: { onComplete?: () => void } = $props();

	let modelReady = $state(false);
	let modelSetupOpen = $state(false);
	let models = $state<ModelListItem[]>([]);
	let progressByModel = $state<Record<string, number>>({});
	let modelSetupError = $state("");

	let permissions = $state<PermissionStatus[]>([]);
	let outputPath = $state("");
	let openHotkey = $state("");
	let dictateHotkey = $state("");
	let inputLabel = $state("Mic");
	let outputLabel = $state("Speaker");
	let saveError = $state("");

	let step = $state<1 | 2 | 3 | 4>(1);

	const canCloseModelSetup = $derived(models.some((m) => m.selected && m.downloaded));
	const permissionsReady = $derived(permissions.every((p) => p.granted || !p.can_request));
	const outputPathReady = $derived(Boolean(outputPath.trim()));
	const hotkeysReady = $derived(Boolean(openHotkey.trim() && dictateHotkey.trim()));
	const allReady = $derived(modelReady && permissionsReady && outputPathReady && hotkeysReady);

	async function refreshModels() {
		models = await invoke<ModelListItem[]>("model_list").catch(() => []);
		modelReady = models.some((m) => m.downloaded && m.selected);
	}

	async function refreshPermissions() {
		permissions = await invoke<PermissionStatus[]>("settings_permissions_status").catch(() => []);
	}

	async function refreshConfig() {
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

	async function openPermissionSettings(kind: string) {
		await invoke("settings_permissions_open", { kind }).catch(() => {});
		await refreshPermissions();
	}

	async function saveOutputPath() {
		saveError = "";
		await invoke("settings_set_output_path", { path: outputPath }).catch((e) => {
			saveError = String(e);
		});
		await refreshConfig();
	}

	async function saveHotkeysAndLabels() {
		saveError = "";
		await invoke("settings_set_hotkeys", {
			openScribe: openHotkey,
			dictate: dictateHotkey,
		}).catch((e) => {
			saveError = String(e);
		});
		await invoke("settings_set_input_labels", {
			inputLabel,
			outputLabel,
		}).catch((e) => {
			saveError = String(e);
		});
		await refreshConfig();
	}

	async function downloadModel(modelId: string) {
		modelSetupError = "";
		await invoke("model_download", { modelId }).catch((e) => {
			modelSetupError = String(e);
		});
	}

	async function selectModel(modelId: string) {
		modelSetupError = "";
		await invoke("model_select", { modelId }).catch((e) => {
			modelSetupError = String(e);
		});
		await refreshModels();
	}

	onMount(async () => {
		await Promise.all([refreshModels(), refreshPermissions(), refreshConfig()]);
	});
</script>

<div class="mx-auto flex h-screen w-full max-w-3xl flex-col gap-6 p-6 text-on-surface">
	<header class="space-y-1">
		<h1 class="text-title-lg font-semibold">Welcome to Liscribe</h1>
		<p class="text-body-sm text-on-surface/70">
			Finish setup once: choose model, check permissions, confirm output path, then configure hotkeys.
		</p>
	</header>

	<nav class="flex gap-2">
		<Button variant={step === 1 ? "primary" : "secondary"} onclick={() => (step = 1)}>1. Model</Button>
		<Button variant={step === 2 ? "primary" : "secondary"} onclick={() => (step = 2)}
			>2. Permissions</Button
		>
		<Button variant={step === 3 ? "primary" : "secondary"} onclick={() => (step = 3)}>3. Output</Button>
		<Button variant={step === 4 ? "primary" : "secondary"} onclick={() => (step = 4)}>4. Hotkeys</Button>
	</nav>

	<section class="rounded-md border border-surface-container p-4">
		{#if step === 1}
			<div class="space-y-3">
				<p class="text-body-sm">Select and download a model for transcription.</p>
				<Button variant="primary" onclick={async () => {
					modelSetupOpen = true;
					await refreshModels();
				}}>Open model setup</Button>
				{#if modelReady}
					<p class="text-label-sm text-on-surface/70">Model setup complete.</p>
				{/if}
			</div>
		{:else if step === 2}
			<div class="space-y-3">
				{#each permissions as permission (permission.kind)}
					<div class="flex items-center justify-between">
						<p class="text-body-sm capitalize">{permission.kind.replace("_", " ")}</p>
						{#if permission.granted}
							<span class="text-label-sm text-green-500">Granted</span>
						{:else if permission.can_request}
							<Button
								variant="secondary"
								onclick={() => openPermissionSettings(permission.kind)}
							>
								Open Settings
							</Button>
						{:else}
							<span class="text-label-sm text-on-surface/70">Unavailable</span>
						{/if}
					</div>
				{/each}
			</div>
		{:else if step === 3}
			<div class="space-y-3">
				<PathSelectorField
					label="Output folder"
					bind:path={outputPath}
					onChange={saveOutputPath}
				/>
				<p class="text-label-sm text-on-surface/70">Current path must exist or be creatable.</p>
			</div>
		{:else}
			<div class="space-y-3">
				<ConfigField label="Open Scribe hotkey" mode="action" bind:value={openHotkey} />
				<ConfigField label="Dictate hotkey" mode="action" bind:value={dictateHotkey} />
				<LabeledTextField label="Input label" bind:value={inputLabel} />
				<LabeledTextField label="Output label" bind:value={outputLabel} />
				<Button variant="primary" onclick={saveHotkeysAndLabels}>Save hotkeys and labels</Button>
			</div>
		{/if}
		{#if saveError}
			<p class="text-label-sm text-error">{saveError}</p>
		{/if}
	</section>

	<footer class="mt-auto flex items-center justify-between">
		<Button variant="secondary" onclick={async () => {
			await Promise.all([refreshModels(), refreshPermissions(), refreshConfig()]);
		}}>Refresh status</Button>
		<Button variant="primary" disabled={!allReady} onclick={() => onComplete?.()}>Enter Scribe</Button>
	</footer>
</div>

<ModelSetupModal
	open={modelSetupOpen}
	{models}
	{progressByModel}
	errorMessage={modelSetupError}
	canClose={canCloseModelSetup}
	onDownload={downloadModel}
	onSelect={selectModel}
	onClose={() => (modelSetupOpen = false)}
/>
