<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { createModelDownloadStore } from '$lib/stores/modelDownload.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import type { ToastState } from '$lib/components/Toast.svelte';
	import Button from '$lib/components/Button.svelte';
	import IconButton from '$lib/components/IconButton.svelte';
	import { CircleCheckBig, Download, RefreshCw } from 'lucide-svelte';

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
		modelsRefreshed: { message: 'Models refreshed', state: 'success' },
	} satisfies Record<string, ToastConfig>;

	let unlisteners: (() => void)[] = [];
	let toast = $state<ToastConfig>({ ...emptyToast });
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;
	let refreshing = $state(false);

	const selectedModel = $derived(modelStore.models.find((m) => m.selected));
	const selectedId = $derived(selectedModel?.id ?? '');
	const downloadedModels = $derived(modelStore.models.filter((m) => m.downloaded));
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

	async function onRefresh() {
		clearToast();
		refreshing = true;
		try {
			await modelStore.refresh();
			if (!modelStore.error && showToast) showToastMessage(toastMessages.modelsRefreshed);
		} finally {
			refreshing = false;
		}
	}

	function onSelectChange(ev: Event) {
		const el = ev.currentTarget as HTMLSelectElement;
		const value = el.value;
		if (value) selectModel(value);
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
	{#if showHeader}
		<h2 class="sf-headline-sm shrink-0 px-4 pt-4">{heading}</h2>
	{/if}

	{#if modelStore.error}
		<p
			class={`rounded-md border border-fill px-3 py-2 text-body-md text-destructive ${showHeader ? 'mx-4 mt-3' : 'mx-4 mt-4'}`}
		>
			{modelStore.error}
		</p>
	{/if}

	<!-- Active model — Scribe -->
	<div
		class={`shrink-0 border-b border-card bg-panel px-4 py-3 ${showHeader ? '' : 'pt-4'}`}
	>
		<h3 class="text-label-sm font-medium uppercase tracking-stamped text-fg-dim">Active model</h3>
		<div class="mt-2 flex flex-col gap-2">
			<div
				class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 text-body-md text-fg-dim"
			>
				<div class="flex min-w-0 items-center gap-2">
					<span
						class="min-w-[2.75rem] shrink-0 rounded-sm border border-brand bg-brand/10 px-1.5 py-0.5 text-center font-sans text-[0.5625rem] font-semibold uppercase tracking-stamped text-brand"
					>
						Scribe
					</span>
					<span class="truncate">Used for transcription</span>
				</div>
				<div class="flex shrink-0 flex-col items-end gap-0.5">
					<label class="sr-only" for="scribe-model-select">Scribe transcription model</label>
					<select
						id="scribe-model-select"
						class="h-8 min-w-[10rem] max-w-[14rem] cursor-pointer truncate rounded-md border border-fill bg-panel py-2 pr-8 pl-2 text-body-md text-fg disabled:cursor-not-allowed disabled:opacity-40"
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
					{#if selectedId}
						<span class="font-sans text-[0.5625rem] font-medium uppercase tracking-[0.04em] text-brand">active</span>
					{/if}
				</div>
			</div>
		</div>
	</div>

	<!-- Library -->
	<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
		<div class="flex items-center justify-between gap-2">
			<h3 class="text-label-sm font-medium uppercase tracking-stamped text-fg-dim">
				Installed &amp; available
			</h3>
			<IconButton
				icon={RefreshCw}
				variant="normal"
				size="small"
				aria-label="Refresh model list"
				disabled={refreshing}
				iconExtraClass={refreshing ? 'animate-spin' : ''}
				onclick={() => void onRefresh()}
			/>
		</div>

		<div class="mt-2 overflow-hidden rounded-md border border-fill bg-card">
			<!-- Column headers -->
			<div
				class="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2 border-b border-fill bg-canvas px-2.5 py-1"
			>
				<span class="font-sans text-[0.5625rem] font-semibold uppercase tracking-stamped text-fg-dim">Model</span>
				<span class="w-8 text-center font-sans text-[0.5625rem] font-semibold uppercase tracking-stamped text-fg-dim">OK</span>
				<span
					class="min-w-[4.75rem] text-right font-sans text-[0.5625rem] font-semibold uppercase tracking-stamped text-fg-dim"
					title="Install or switch model">↓</span>
			</div>

			{#each modelStore.models as model (model.id)}
				<div
					class="grid grid-cols-[minmax(0,1fr)_auto_auto] items-start gap-2 border-b border-fill px-2.5 py-2 last:border-b-0"
				>
					<div class="min-w-0">
						<div class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5">
							<span class={`text-label-md font-sans ${model.downloaded ? 'font-medium text-fg' : 'font-normal text-fg-dim'}`}>{model.label}</span>
							{#if model.selected && model.downloaded}
								<span
									class="shrink-0 rounded-sm border border-brand bg-brand/10 px-1 py-px font-sans text-[0.5625rem] font-semibold uppercase tracking-stamped text-brand"
								>
									scribe
								</span>
							{/if}
						</div>
						{#if model.file_name}
							<p class="mt-0.5 truncate font-mono text-[0.6rem] leading-tight text-fg-dim">{model.file_name}</p>
						{/if}
						{#if !model.downloaded && (rowDownloading(model.id) || (modelStore.progressByModel[model.id] ?? 0) > 0)}
							{@const pct = progressPct(model.id)}
							<div class="mt-2 h-0.5 w-full overflow-hidden rounded-sm bg-fill">
								<div
									class="h-full rounded-sm bg-fg-dim transition-[width]"
									style={`width:${pct}%`}
								></div>
							</div>
							{#if pct < 100}
								<p class="mt-2 font-mono text-[0.6rem] leading-tight text-fg-dim">{pct}%</p>
							{:else}
								<p class="mt-2 font-mono text-[0.6rem] leading-tight text-fg-dim">Finalising…</p>
							{/if}
						{/if}
						{#if modelStore.statusByModel[model.id] && !(!model.downloaded && rowDownloading(model.id))}
							<p class="mt-1 font-mono text-[0.6rem] leading-tight text-fg-dim">
								{modelStore.statusByModel[model.id]}
							</p>
						{/if}
					</div>

					<div class="flex w-8 shrink-0 items-start justify-center pt-1">
						{#if model.downloaded && !rowDownloading(model.id)}
							{#if model.selected}
								<span class="flex text-fg-dim">
									<CircleCheckBig class="size-[11px]" strokeWidth={2.5} />
								</span>
							{:else}
								<span aria-hidden="true" class="text-fg-muted">—</span>
							{/if}
						{:else if rowDownloading(model.id)}
							<span class="sr-only">Installing…</span>
							<span class="size-2 shrink-0 rounded-full bg-fg-dim animate-pulse" aria-hidden="true"></span>
						{/if}
					</div>

					<div class="flex min-w-[4.75rem] shrink-0 items-start justify-end pt-0.5">
						{#if model.downloaded}
							{#if rowDownloading(model.id)}
								<IconButton
									icon={Download}
									variant="normal"
									size="small"
									disabled={true}
									aria-label={`Finishing ${model.label} install`}
								/>
							{:else if model.selected}
								<span class="pointer-events-none whitespace-nowrap rounded-md border border-fill px-2 py-0.5 text-label-md text-fg-dim">
									In use
								</span>
							{:else}
								<Button variant="normal" size="small" class="whitespace-nowrap" onclick={() => void selectModel(model.id)}>
									Use model
								</Button>
							{/if}
						{:else}
							<IconButton
								icon={Download}
								variant="normal"
								size="small"
								disabled={!!modelStore.downloadingByModel[model.id]}
								aria-label={`Install ${model.label}`}
								onclick={() => void downloadModel(model.id)}
							/>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

{#if showToast}
	<Toast message={toast.message} state={toast.state} />
{/if}
