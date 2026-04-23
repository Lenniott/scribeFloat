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
	let gateError = $state("");
	let isRefreshing = $state(false);

	let step = $state<1 | 2 | 3 | 4>(1);

	const canCloseModelSetup = $derived(models.some((m) => m.selected && m.downloaded));
	const permissionsReady = $derived(permissions.every((p) => p.granted || !p.can_request));
	const outputPathReady = $derived(Boolean(outputPath.trim()));
	const hotkeysReady = $derived(Boolean(openHotkey.trim() && dictateHotkey.trim()));
	const allReady = $derived(modelReady && permissionsReady && outputPathReady && hotkeysReady);
	const stepStatuses = $derived([
		{
			id: 1 as const,
			title: "Model",
			description: modelReady ? "Ready" : "Download and select one model",
			complete: modelReady,
		},
		{
			id: 2 as const,
			title: "Permissions",
			description: permissionsReady ? "Ready" : "Grant required system permissions",
			complete: permissionsReady,
		},
		{
			id: 3 as const,
			title: "Output",
			description: outputPathReady ? "Ready" : "Choose output folder",
			complete: outputPathReady,
		},
		{
			id: 4 as const,
			title: "Hotkeys",
			description: hotkeysReady ? "Ready" : "Set scribe and dictate hotkeys",
			complete: hotkeysReady,
		},
	]);

	function getErrorMessage(error: unknown, fallback: string): string {
		if (typeof error === "string" && error.trim()) return error;
		if (error instanceof Error && error.message.trim()) return error.message;
		if (typeof error === "object" && error !== null) {
			const maybeMessage = (error as { message?: unknown }).message;
			if (typeof maybeMessage === "string" && maybeMessage.trim()) return maybeMessage;
		}
		return fallback;
	}

	async function refreshModels() {
		models = await invoke<ModelListItem[]>("model_list").catch((error) => {
			gateError = getErrorMessage(error, "Could not load model status.");
			return [];
		});
		modelReady = models.some((m) => m.downloaded && m.selected);
	}

	async function refreshPermissions() {
		permissions = await invoke<PermissionStatus[]>("settings_permissions_status").catch((error) => {
			gateError = getErrorMessage(error, "Could not load permission status.");
			return [];
		});
	}

	async function refreshConfig() {
		outputPath = await invoke<string>("settings_get_output_path").catch((error) => {
			gateError = getErrorMessage(error, "Could not load output path.");
			return "";
		});
		const [open, dictate] = await invoke<[string, string]>("settings_get_hotkeys").catch((error) => {
			gateError = getErrorMessage(error, "Could not load hotkeys.");
			return ["", ""];
		});
		openHotkey = open;
		dictateHotkey = dictate;
		const [inLabel, outLabel] = await invoke<[string, string]>("settings_get_input_labels").catch(
			(error) => {
				gateError = getErrorMessage(error, "Could not load labels.");
				return ["Mic", "Speaker"];
			},
		);
		inputLabel = inLabel;
		outputLabel = outLabel;
	}

	async function openPermissionSettings(kind: string) {
		await invoke("settings_permissions_open", { kind }).catch((error) => {
			saveError = getErrorMessage(error, `Could not open settings for ${kind.replace("_", " ")}.`);
		});
		await refreshPermissions();
	}

	async function saveOutputPath() {
		saveError = "";
		await invoke("settings_set_output_path", { path: outputPath }).catch((e) => {
			saveError = getErrorMessage(e, "Could not save output path.");
		});
		await refreshConfig();
	}

	async function saveHotkeysAndLabels() {
		saveError = "";
		await invoke("settings_set_hotkeys", {
			openScribe: openHotkey,
			dictate: dictateHotkey,
		}).catch((e) => {
			saveError = getErrorMessage(e, "Could not save hotkeys.");
		});
		await invoke("settings_set_input_labels", {
			inputLabel,
			outputLabel,
		}).catch((e) => {
			saveError = getErrorMessage(e, "Could not save labels.");
		});
		await refreshConfig();
	}

	async function downloadModel(modelId: string) {
		modelSetupError = "";
		await invoke("model_download", { modelId }).catch((e) => {
			modelSetupError = getErrorMessage(e, "Could not start model download.");
		});
	}

	async function selectModel(modelId: string) {
		modelSetupError = "";
		await invoke("model_select", { modelId }).catch((e) => {
			modelSetupError = getErrorMessage(e, "Could not select model.");
		});
		await refreshModels();
	}

	async function refreshAllStatus() {
		isRefreshing = true;
		gateError = "";
		await Promise.all([refreshModels(), refreshPermissions(), refreshConfig()]);
		isRefreshing = false;
	}

	onMount(async () => {
		await refreshAllStatus();
	});
</script>

<div class="mx-auto flex h-screen w-full max-w-3xl flex-col gap-6 p-6 text-on-surface">
	<header class="space-y-1">
		<h1 class="text-title-lg font-semibold">Welcome to Liscribe</h1>
		<p class="text-body-sm text-on-surface/70">
			Finish setup once: choose model, check permissions, confirm output path, then configure hotkeys.
		</p>
	</header>

	<nav class="grid gap-2 sm:grid-cols-2">
		{#each stepStatuses as stepStatus (stepStatus.id)}
			<button
				type="button"
				class={`rounded-md border px-3 py-2 text-left transition ${
					stepStatus.id === step
						? "border-primary bg-primary/10"
						: stepStatus.complete
							? "border-green-500/40 bg-green-500/10"
							: "border-surface-container bg-surface"
				}`}
				onclick={() => (step = stepStatus.id)}
			>
				<div class="flex items-center justify-between gap-3">
					<p class="text-label-sm font-semibold">
						{stepStatus.id}. {stepStatus.title}
					</p>
					<span class={`text-label-sm ${stepStatus.complete ? "text-green-500" : "text-on-surface/70"}`}>
						{stepStatus.complete ? "Done" : "Pending"}
					</span>
				</div>
				<p class="text-label-sm text-on-surface/70">{stepStatus.description}</p>
			</button>
		{/each}
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
				{:else}
					<p class="text-label-sm text-on-surface/70">Choose a downloaded model to continue.</p>
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
		{#if gateError}
			<p class="text-label-sm text-error">{gateError}</p>
		{/if}
		{#if saveError}
			<p class="text-label-sm text-error">{saveError}</p>
		{/if}
	</section>

	<footer class="mt-auto flex items-center justify-between">
		<Button variant="secondary" disabled={isRefreshing} onclick={refreshAllStatus}>
			{isRefreshing ? "Refreshing..." : "Refresh status"}
		</Button>
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
