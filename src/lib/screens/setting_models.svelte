<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { createModelDownloadStore } from '$lib/stores/modelDownload.svelte';
	import Toast from '@lib/components/Toast.svelte';
	import type { ToastState } from '@lib/components/Toast.svelte';
	import IconButton from '@lib/components/IconButton.svelte';
	import { Download, RefreshCw, Trash2 } from 'lucide-svelte';

	let {
		ready = $bindable(false),
	}: {
		ready?: boolean;
	} = $props();

	type ToastConfig = {
		message: string;
		state: ToastState;
	};

	const modelStore = createModelDownloadStore();
	const emptyToast: ToastConfig = { message: '', state: 'normal' };
	const toastMessages = {
		modelSelected: { message: 'Model selected', state: 'success' },
		modelsRefreshed: { message: 'Models refreshed', state: 'success' },
		modelRemoved: { message: 'Model removed', state: 'success' },
	} satisfies Record<string, ToastConfig>;

	let unlisteners: (() => void)[] = [];
	let toast = $state<ToastConfig>({ ...emptyToast });
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;
	let refreshing = $state(false);
	/** Avoid pushing `ready=false` while the store list is still empty before first refresh. */
	let readyHydrated = $state(false);

	const selectedModel = $derived(modelStore.models.find((m) => m.selected));
	const selectedId = $derived(selectedModel?.id ?? '');
	const downloadedModels = $derived(modelStore.models.filter((m) => m.downloaded));
	const hasReadyModel = $derived(modelStore.models.some((m) => m.selected && m.downloaded));

	/** `null` means "follow Scribe default"; non-null means an explicit override. */
	let dictateModelId = $state<string | null>(null);

	$effect(() => {
		if (!readyHydrated) return;
		ready = hasReadyModel;
	});

	onMount(async () => {
		unlisteners = await modelStore.subscribe();
		await modelStore.refresh();
		dictateModelId = await invoke<string | null>('settings_get_dictate_model_id').catch(() => null);
		readyHydrated = true;
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
		if (!modelStore.error) showToastMessage(toastMessages.modelSelected);
	}

	async function onRefresh() {
		if (refreshing) return;
		clearToast();
		refreshing = true;
		const minSpinMs = 550;
		const started = typeof performance !== 'undefined' ? performance.now() : Date.now();
		try {
			await modelStore.refresh();
			if (!modelStore.error) showToastMessage(toastMessages.modelsRefreshed);
		} finally {
			const elapsed =
				(typeof performance !== 'undefined' ? performance.now() : Date.now()) - started;
			if (elapsed < minSpinMs) {
				await new Promise((r) => setTimeout(r, minSpinMs - elapsed));
			}
			refreshing = false;
		}
	}

	async function removeModel(modelId: string) {
		clearToast();
		await modelStore.remove(modelId);
		if (!modelStore.error) showToastMessage(toastMessages.modelRemoved);
	}

	function onSelectChange(ev: Event) {
		const el = ev.currentTarget as HTMLSelectElement;
		const value = el.value;
		if (value) selectModel(value);
	}

	async function onDictateSelectChange(ev: Event) {
		const el = ev.currentTarget as HTMLSelectElement;
		const value = el.value || null;
		dictateModelId = value;
		try {
			await invoke('settings_set_dictate_model_id', { modelId: value });
			showToastMessage(toastMessages.modelSelected);
		} catch (e) {
			modelStore.error = String(e);
		}
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

	function progressPct(modelId: string): number {
		return Math.round((modelStore.progressByModel[modelId] ?? 0) * 100);
	}

	function rowDownloading(modelId: string): boolean {
		return !!modelStore.downloadingByModel[modelId];
	}
</script>

<div class="flex h-full min-h-0 flex-1 flex-col">
	<h2 class="sf-headline-sm shrink-0 p-4">Whisper models</h2>

	{#if modelStore.error}
		<p class="rounded-md border border-fill px-3 py-2 sf-body-md text-destructive mx-4 mt-3">
			{modelStore.error}
		</p>
	{/if}

	<!-- Transcription defaults — Scribe + Dictate -->
	<div class="shrink-0 border-b border-card bg-panel px-4 py-3">
		<h3 class="sf-label-sm text-fg-dim">Default models</h3>
		<div class="mt-2 flex flex-col gap-2">
			<!-- Scribe -->
			<div class="sf-body-md flex flex-wrap items-center justify-between gap-x-4 gap-y-2 text-fg-dim">
				<div class="flex min-w-0 items-center gap-2">
					<span
						class="sf-label-sm shrink-0 rounded-sm border border-brand bg-brand/10 px-1.5 py-0.5 text-center min-w-11 text-brand"
					>
						Scribe
					</span>
				</div>
				<div class="flex shrink-0 flex-col items-end gap-0.5">
					<label class="sr-only" for="scribe-model-select">Default Scribe transcription model</label>
					<select
						id="scribe-model-select"
						class="sf-body-md h-8 min-w-40 max-w-56 cursor-pointer truncate rounded-md border border-fill bg-panel py-2 pr-8 pl-2 text-fg disabled:cursor-not-allowed disabled:opacity-40"
						value={selectedId}
						onchange={onSelectChange}
						disabled={downloadedModels.length === 0}
					>
						{#if !selectedId && downloadedModels.length > 0}
							<option value="" disabled>Choose model…</option>
						{/if}
						{#each downloadedModels as dm (dm.id)}
							<option value={dm.id}>{dm.label}</option>
						{/each}
					</select>
				</div>
			</div>
			<!-- Dictate -->
			<div class="sf-body-md flex flex-wrap items-center justify-between gap-x-4 gap-y-2 text-fg-dim">
				<div class="flex min-w-0 items-center gap-2">
					<span
						class="sf-label-sm shrink-0 rounded-sm border border-focus bg-focus/15 px-1 py-px text-focus"
					>
						Dictate
					</span>
				</div>
				<div class="flex shrink-0 flex-col items-end gap-0.5">
					<label class="sr-only" for="dictate-model-select">Dictate transcription model override</label>
					<select
						id="dictate-model-select"
						class="sf-body-md h-8 min-w-40 max-w-56 cursor-pointer truncate rounded-md border border-fill bg-panel py-2 pr-8 pl-2 text-fg disabled:cursor-not-allowed disabled:opacity-40"
						value={dictateModelId ?? ''}
						onchange={onDictateSelectChange}
						disabled={downloadedModels.length === 0}
					>
						<option value="">Same as Scribe</option>
						{#each downloadedModels as dm (dm.id)}
							<option value={dm.id}>{dm.label}</option>
						{/each}
					</select>
				</div>
			</div>
		</div>
	</div>

	<!-- Library -->
	<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
		<div class="flex items-center justify-between gap-2">
			<h3 class="sf-label-sm text-fg-dim">Installed &amp; available</h3>
			<IconButton
				icon={RefreshCw}
				variant="normal"
				size="small"
				aria-label="Refresh model list"
				class={refreshing ? 'pointer-events-none' : ''}
				iconExtraClass={refreshing ? 'motion-safe:animate-spin' : ''}
				onclick={() => void onRefresh()}
			/>
		</div>

		<div class="mt-2 divide-y divide-fill overflow-hidden rounded-md border border-fill bg-panel">
			{#each modelStore.models as model (model.id)}
				<div class="flex items-center justify-between gap-3 px-3 py-2.5">
					<div class="min-w-0 flex-1 items-center">
						<div class="flex min-w-0 items-baseline gap-x-2 gap-y-0.5">
							<span
								class={model.downloaded
									? 'sf-label-md text-fg'
									: 'text-label-md font-sans font-normal text-fg-dim'}>{model.label}</span
							>

							{#if !model.downloaded && (rowDownloading(model.id) || (modelStore.progressByModel[model.id] ?? 0) > 0)}
								{@const pct = progressPct(model.id)}
								<div class="h-full w-full flex items-center gap-1">
									<div class="h-0.5 w-full overflow-hidden rounded-sm bg-fill">
										<div
											class="h-full rounded-sm bg-active transition-[width]"
											style={`width:${pct}%`}
										></div>
									</div>
									{#if pct < 100}
										<p class="font-mono text-label-md font-normal leading-snug text-active">
											{pct}%
										</p>
									{:else}
										<p class="font-mono text-label-md font-normal leading-snug text-active">
											Finalising…
										</p>
									{/if}
								</div>
							{/if}
						{#if model.selected && model.downloaded}
							<span
								class="sf-label-sm shrink-0 rounded-sm border border-brand bg-brand/10 px-1 py-px text-brand"
							>
								scribe
							</span>
						{/if}
						{#if model.downloaded && (model.id === dictateModelId || (dictateModelId === null && model.selected))}
							<span
								class="sf-label-sm shrink-0 rounded-sm border border-focus bg-focus/15 px-1 py-px text-focus"
							>
								dictate
							</span>
						{/if}
						</div>
					</div>

					<div class="flex shrink-0 items-start justify-end pt-0.5">
						{#if model.downloaded && !rowDownloading(model.id)}
							<IconButton
								icon={Trash2}
								variant="destructive"
								size="small"
								aria-label={`Remove ${model.label}`}
								onclick={() => void removeModel(model.id)}
							/>
						{:else}
							<IconButton
								icon={Download}
								variant="normal"
								size="small"
								disabled={!!modelStore.downloadingByModel[model.id]}
								aria-label={rowDownloading(model.id)
									? `Installing ${model.label}`
									: `Install ${model.label}`}
								onclick={() => void downloadModel(model.id)}
							/>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

<Toast message={toast.message} state={toast.state} />
