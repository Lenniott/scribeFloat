<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { createModelDownloadStore } from '$lib/stores/modelDownload.svelte';
	import Button from '@lib/components/Button.svelte';
	import Toast from '@lib/components/Toast.svelte';
	import type { ToastState } from '@lib/components/Toast.svelte';
	import { CircleCheckBig, Download } from 'lucide-svelte';

	type ToastConfig = {
		message: string;
		state: ToastState;
	};

	let {
		heading = 'Whisper models',
		showHeader = true,
		showToast = true,
		ready = $bindable(false),
	}: {
		heading?: string;
		showHeader?: boolean;
		showToast?: boolean;
		ready?: boolean;
	} = $props();

	const modelStore = createModelDownloadStore();
	const emptyToast: ToastConfig = { message: '', state: 'normal' };
	const toastMessages = {
		modelSelected: { message: 'Model selected', state: 'success' },
	} satisfies Record<string, ToastConfig>;

	let unlisteners: (() => void)[] = [];
	let toast = $state<ToastConfig>({ ...emptyToast });
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;

	const selectedModel = $derived(modelStore.models.find((m) => m.selected));
	const hasReadyModel = $derived(modelStore.models.some((m) => m.selected && m.downloaded));

	$effect(() => {
		ready = hasReadyModel;
	});

	onMount(async () => {
		unlisteners = await modelStore.subscribe();
		await modelStore.refresh();
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
		if (toastTimeout) clearTimeout(toastTimeout);
	});

	async function downloadModel(modelId: string) {
		clearToast();
		await modelStore.download(modelId);
	}

	async function selectModel(modelId: string) {
		clearToast();
		await modelStore.select(modelId);
		if (!modelStore.error && showToast) showToastMessage(toastMessages.modelSelected);
	}

	function clearToast() {
		toast = { ...emptyToast };
	}

	function showToastMessage(nextToast: ToastConfig) {
		if (toastTimeout) clearTimeout(toastTimeout);
		toast = nextToast;
		toastTimeout = setTimeout(() => {
			clearToast();
			toastTimeout = null;
		}, 2000);
	}
</script>

<section class="space-y-4 h-full">
	{#if showHeader}
		<h2 class="sf-headline-sm">{heading}</h2>
	{/if}

	{#if selectedModel}
		<p class="text-body-md text-on-surface/70">
			Active Scribe model:
			<span class="font-normal text-on-surface">{selectedModel.label}</span>
		</p>
	{:else}
		<p class="text-body-md text-on-surface/70">No model selected yet.</p>
	{/if}

	{#if modelStore.error}
		<p class="rounded-md border border-surface-high px-3 py-2 text-body-md text-error">
			{modelStore.error}
		</p>
	{/if}
	<div class="space-y-3">
		{#each modelStore.models as model (model.id)}
			<div
				class={`rounded-md border ${model.selected ? 'border-surface-highest' : 'border-surface-high'} px-3 py-3`}
			>
				<div class="flex items-center">
					<div class="flex min-w-0 grow items-center gap-2">
						<p class="text-label-md font-normal text-on-surface">
							{model.label}
						</p>
						{#if modelStore.statusByModel[model.id]}
							<p class="text-label-md text-on-surface/70">
								{modelStore.statusByModel[model.id]}
							</p>
						{/if}
					</div>
					<div class="flex items-center gap-2">
						{#if model.downloaded}
							{#if model.selected}
								<span class="flex size-6 items-center justify-center p-1 text-on-surface">
									<CircleCheckBig />
								</span>
							{:else}
								<Button variant="normal" onclick={() => selectModel(model.id)}>
									Use model
								</Button>
							{/if}
						{:else}
							<Button
								variant="ghost"
								disabled={!!modelStore.downloadingByModel[model.id]}
								onclick={() => downloadModel(model.id)}
								icon={Download}
							>
								{modelStore.downloadingByModel[model.id] ? 'Installing…' : 'Install'}
							</Button>
						{/if}
					</div>
				</div>
				{#if !model.downloaded && (modelStore.downloadingByModel[model.id] || (modelStore.progressByModel[model.id] ?? 0) > 0)}
					<div class="h-2 w-full overflow-hidden rounded bg-surface-high">
						<div
							class="h-full bg-on-surface-dim transition-all"
							style={`width:${Math.round((modelStore.progressByModel[model.id] ?? 0) * 100)}%`}
						></div>
					</div>
					<p class="mt-1 text-label-sm text-on-surface/70">
						{Math.round((modelStore.progressByModel[model.id] ?? 0) * 100)}%
					</p>
				{/if}
			</div>
		{/each}
	</div>
</section>

{#if showToast}
	<Toast message={toast.message} state={toast.state} />
{/if}
