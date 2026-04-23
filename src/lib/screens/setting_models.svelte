<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import Button from "@lib/components/Button.svelte";

	type ModelListItem = {
		id: string;
		label: string;
		file_name: string;
		downloaded: boolean;
		selected: boolean;
	};

	type ModelProgressPayload = {
		model_id: string;
		progress: number;
		bytes_downloaded: number;
		total_bytes?: number;
	};

	let models = $state<ModelListItem[]>([]);
	let progressByModel = $state<Record<string, number>>({});
	let errorMessage = $state("");
	let statusMessage = $state("");
	let unlisteners: UnlistenFn[] = [];

	const selectedModel = $derived(models.find((m) => m.selected));

	async function refreshModels() {
		models = await invoke<ModelListItem[]>("model_list").catch(() => []);
	}

	async function downloadModel(modelId: string) {
		errorMessage = "";
		statusMessage = `Downloading ${modelId}...`;
		await invoke("model_download", { modelId }).catch((e) => {
			errorMessage = String(e);
			statusMessage = "";
		});
	}

	async function selectModel(modelId: string) {
		errorMessage = "";
		statusMessage = "";
		await invoke("model_select", { modelId }).catch((e) => {
			errorMessage = String(e);
		});
		await refreshModels();
	}

	onMount(async () => {
		await refreshModels();
		const ul1 = await listen<ModelProgressPayload>("model://download-progress", (e) => {
			progressByModel = { ...progressByModel, [e.payload.model_id]: e.payload.progress };
			if (e.payload.progress >= 1) {
				statusMessage = `${e.payload.model_id} downloaded`;
				refreshModels();
			}
		});
		const ul2 = await listen<string>("model://download-error", (e) => {
			errorMessage = e.payload ?? "Model download failed";
			statusMessage = "";
		});
		unlisteners = [ul1, ul2];
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
	});
</script>

<section class="space-y-4">
	<div class="flex items-center justify-between">
		<h2 class="text-title-sm font-semibold">Whisper models</h2>
		<Button variant="secondary" onclick={refreshModels}>Refresh</Button>
	</div>

	{#if selectedModel}
		<p class="text-body-sm text-on-surface/70">
			Active Scribe model: <span class="font-semibold text-on-surface">{selectedModel.label}</span>
		</p>
	{:else}
		<p class="text-body-sm text-on-surface/70">No model selected yet.</p>
	{/if}

	{#if errorMessage}
		<p class="rounded-md bg-error-container/20 px-3 py-2 text-body-sm text-error">{errorMessage}</p>
	{/if}
	{#if statusMessage}
		<p class="rounded-md bg-surface-container-low px-3 py-2 text-body-sm text-on-surface/80">
			{statusMessage}
		</p>
	{/if}

	<div class="space-y-3">
		{#each models as model (model.id)}
			<div class="rounded-md border border-surface-container-high px-3 py-3">
				<div class="mb-2 flex items-center justify-between gap-2">
					<div class="min-w-0">
						<p class="text-label-md font-semibold text-on-surface">{model.label}</p>
						<p class="truncate text-body-sm text-on-surface/60">{model.file_name}</p>
					</div>
					<div class="flex items-center gap-2">
						{#if model.downloaded}
							<Button
								variant={model.selected ? "primary" : "secondary"}
								onclick={() => selectModel(model.id)}
							>
								{model.selected ? "Selected" : "Use model"}
							</Button>
						{:else}
							<Button variant="primary" onclick={() => downloadModel(model.id)}>Download</Button>
						{/if}
					</div>
				</div>
				{#if !model.downloaded}
					<div class="h-2 w-full overflow-hidden rounded bg-surface-container-high">
						<div
							class="h-full bg-primary transition-all"
							style={`width:${Math.round((progressByModel[model.id] ?? 0) * 100)}%`}
						></div>
					</div>
					<p class="mt-1 text-label-sm text-on-surface/70">
						{Math.round((progressByModel[model.id] ?? 0) * 100)}%
					</p>
				{/if}
			</div>
		{/each}
	</div>
</section>