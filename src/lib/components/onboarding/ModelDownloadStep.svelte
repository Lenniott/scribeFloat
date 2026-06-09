<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { createModelDownloadStore } from "$lib/stores/modelDownload.svelte";
	import Button from "@lib/components/Button.svelte";
	import IconButton from "@lib/components/IconButton.svelte";
	import StepShell from "./StepShell.svelte";
	import { Download, CircleCheckBig } from "lucide-svelte";

	let {
		onNext,
	}: {
		onNext: () => void;
	} = $props();

	const modelStore = createModelDownloadStore();
	let unlisteners: (() => void)[] = [];

	const hasReady = $derived(modelStore.models.some((m) => m.selected && m.downloaded));

	function progressPct(modelId: string): number {
		return Math.round((modelStore.progressByModel[modelId] ?? 0) * 100);
	}

	function isDownloading(modelId: string): boolean {
		return !!modelStore.downloadingByModel[modelId];
	}

	async function download(modelId: string) {
		await modelStore.download(modelId);
	}

	async function selectAndContinue() {
		// Select the first downloaded model if nothing is selected yet
		const downloaded = modelStore.models.filter((m) => m.downloaded);
		if (downloaded.length > 0 && !hasReady) {
			await modelStore.select(downloaded[0].id);
		}
		onNext();
	}

	onMount(async () => {
		unlisteners = await modelStore.subscribe();
		await modelStore.refresh();
	});

	onDestroy(() => unlisteners.forEach((u) => u()));
</script>

<StepShell
	title="Install AI model"
	subtitle="Whisper runs entirely on your device — nothing leaves your computer."
>
	{#snippet children()}
		<div class="space-y-3">
			{#if modelStore.error}
				<p class="text-label-sm text-destructive px-1">{modelStore.error}</p>
			{/if}

			<div class="divide-y divide-fill overflow-hidden rounded-md border border-fill bg-panel">
				{#each modelStore.models as model (model.id)}
					<div class="flex items-center gap-3 px-3 py-2.5">
						<div class="min-w-0 flex-1">
							<div class="flex items-baseline gap-2 flex-wrap">
								<span class={model.downloaded ? "text-body-md text-fg font-medium" : "text-body-md text-fg-dim"}>
									{model.label}
								</span>
								{#if isDownloading(model.id) || (modelStore.progressByModel[model.id] ?? 0) > 0 && !model.downloaded}
									{@const pct = progressPct(model.id)}
									<div class="flex items-center gap-2 flex-1 min-w-24">
										<div class="h-0.5 flex-1 overflow-hidden rounded-sm bg-fill">
											<div
												class="h-full rounded-sm bg-brand transition-[width] duration-200"
												style={`width:${pct}%`}
											></div>
										</div>
										<span class="font-mono text-label-sm text-fg-dim tabular-nums">
											{pct < 100 ? `${pct}%` : "Finalising…"}
										</span>
									</div>
								{/if}
								{#if model.downloaded}
									<CircleCheckBig class="size-3.5 text-success" />
								{/if}
							</div>
						</div>

						<div class="flex shrink-0 items-center gap-3">
							<span class="font-mono text-label-sm text-fg-dim tabular-nums w-16 text-right">
								{model.size_mb} MB
							</span>
							<div class="w-7 flex justify-end">
								{#if !model.downloaded}
									<IconButton
										icon={Download}
										variant="normal"
										size="small"
										disabled={isDownloading(model.id)}
										aria-label={isDownloading(model.id) ? `Installing ${model.label}` : `Install ${model.label}`}
										onclick={() => void download(model.id)}
									/>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>

			<p class="text-label-sm text-fg-dim px-0.5">
				You can manage and switch models any time in Settings.
			</p>
		</div>
	{/snippet}

	{#snippet footer()}
		<div></div>
		<div class="flex items-center gap-2">
			{#if hasReady}
				<Button variant="primary" onclick={selectAndContinue}>Continue</Button>
			{:else}
				<Button variant="ghost" onclick={onNext}>Skip for now</Button>
			{/if}
		</div>
	{/snippet}
</StepShell>
