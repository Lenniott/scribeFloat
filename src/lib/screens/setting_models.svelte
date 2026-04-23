<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { createModelDownloadStore } from '$lib/stores/modelDownload.svelte';
	import Button from '@lib/components/Button.svelte';

	const modelStore = createModelDownloadStore();
	let unlisteners: (() => void)[] = [];
	let installNotification = $state('');

	const selectedModel = $derived(modelStore.models.find((m) => m.selected));

	onMount(async () => {
		unlisteners = await modelStore.subscribe();
		await modelStore.refresh();
	});

	onDestroy(() => unlisteners.forEach((u) => u()));

	async function downloadModel(modelId: string) {
		installNotification = '';
		await modelStore.download(modelId);
	}

	async function selectModel(modelId: string) {
		installNotification = '';
		await modelStore.select(modelId);
		if (!modelStore.error) installNotification = 'Model selected';
	}
</script>

<section class="space-y-4">
	<div class="flex items-center justify-between">
		<h2 class="text-title-sm font-semibold">Whisper models</h2>
		<Button variant="secondary" onclick={modelStore.refresh}>Refresh</Button>
	</div>

	{#if selectedModel}
		<p class="text-body-sm text-on-surface/70">
			Active Scribe model: <span class="font-semibold text-on-surface">{selectedModel.label}</span>
		</p>
	{:else}
		<p class="text-body-sm text-on-surface/70">No model selected yet.</p>
	{/if}

	{#if modelStore.error}
		<p class="rounded-md bg-error-container/20 px-3 py-2 text-body-sm text-error">{modelStore.error}</p>
	{/if}
	{#if installNotification}
		<p class="rounded-md bg-surface-container-low px-3 py-2 text-body-sm text-on-surface/80">
			{installNotification}
		</p>
	{/if}

	<div class="space-y-3">
		{#each modelStore.models as model (model.id)}
			<div class="rounded-md border border-surface-container-high px-3 py-3">
				<div class="mb-2 flex items-center justify-between gap-2">
					<div class="min-w-0">
						<p class="text-label-md font-semibold text-on-surface">{model.label}</p>
					</div>
					<div class="flex items-center gap-2">
						{#if model.downloaded}
							<Button
								variant={model.selected ? 'primary' : 'secondary'}
								onclick={() => selectModel(model.id)}
							>
								{model.selected ? 'Selected' : 'Use model'}
							</Button>
						{:else}
							<Button
								variant="primary"
								disabled={!!modelStore.downloadingByModel[model.id]}
								onclick={() => downloadModel(model.id)}
							>
								{modelStore.downloadingByModel[model.id] ? 'Installing…' : 'Install'}
							</Button>
						{/if}
					</div>
				</div>
				{#if !model.downloaded && (modelStore.downloadingByModel[model.id] || (modelStore.progressByModel[model.id] ?? 0) > 0)}
					<div class="h-2 w-full overflow-hidden rounded bg-surface-container-high">
						<div
							class="h-full bg-primary transition-all"
							style={`width:${Math.round((modelStore.progressByModel[model.id] ?? 0) * 100)}%`}
						></div>
					</div>
					<p class="mt-1 text-label-sm text-on-surface/70">
						{Math.round((modelStore.progressByModel[model.id] ?? 0) * 100)}%
					</p>
				{/if}
				{#if modelStore.statusByModel[model.id]}
					<p class="mt-1 text-label-sm text-on-surface/70">{modelStore.statusByModel[model.id]}</p>
				{/if}
			</div>
		{/each}
	</div>
</section>
