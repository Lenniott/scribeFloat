<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import Button from "@lib/components/Button.svelte";
	import IconButton from "@lib/components/IconButton.svelte";
	import StepShell from "./StepShell.svelte";
	import { Download, CircleCheckBig } from "lucide-svelte";
	import { createModelDownloadStore } from "$lib/stores/modelDownload.svelte";

	let { onNext }: { onNext: () => void } = $props();

	let unlisteners: (() => void)[] = [];
	const modelStore = createModelDownloadStore();

	const hasAnyDownloaded = $derived(modelStore.models.some((m) => m.downloaded));
	const anyDownloading = $derived(Object.values(modelStore.downloadingByModel).some(Boolean));

	async function download(modelId: string) {
		await modelStore.download(modelId);
	}

	async function selectAndContinue() {
		const downloaded = modelStore.models.filter((m) => m.downloaded);
		const hasSelected = downloaded.some((m) => m.selected);
		if (downloaded.length > 0 && !hasSelected) {
			await modelStore.select(downloaded[0].id);
			if (modelStore.error) {
				return;
			}
		}
		onNext();
	}

	onMount(async () => {
		unlisteners = await modelStore.subscribe();
		await modelStore.refresh();
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
	});

	function pct(modelId: string) {
		return Math.round((modelStore.progressByModel[modelId] ?? 0) * 100);
	}
</script>

<StepShell
	title="Install AI model"
	subtitle="Whisper runs entirely on your device — nothing leaves your computer."
>
	{#snippet children()}
		<div class="space-y-3">
			{#if modelStore.error}
				<p class="sf-label-sm text-destructive px-1">{modelStore.error}</p>
			{/if}

			{#if modelStore.models.length === 0}
				<p class="sf-label-sm text-fg-dim">Loading…</p>
			{:else}
				<div class="divide-y divide-fill overflow-hidden rounded-md border border-fill bg-panel">
					{#each modelStore.models as model (model.id)}
						<div class="flex items-center gap-3 px-3 py-2.5">
							<div class="min-w-0 flex-1">
								<div class="flex items-baseline gap-2 flex-wrap">
									<span class={model.downloaded ? "sf-body-md-strong text-fg" : "sf-body-md text-fg-dim"}>
										{model.label}
									</span>
									{#if modelStore.downloadingByModel[model.id] || ((modelStore.progressByModel[model.id] ?? 0) > 0 && !model.downloaded)}
										{@const p = pct(model.id)}
										<div class="flex items-center gap-2 flex-1 min-w-24">
											<div class="h-0.5 flex-1 overflow-hidden rounded-sm bg-fill">
												<div
													class="h-full rounded-sm bg-brand transition-[width] duration-200"
													style={`width:${p}%`}
												></div>
											</div>
											<span class="sf-meta-sm text-fg-dim">
												{p < 100 ? `${p}%` : "Finalising…"}
											</span>
										</div>
									{/if}
									{#if model.downloaded}
										<CircleCheckBig class="size-3.5 text-success" />
									{/if}
								</div>
							</div>

							<div class="flex shrink-0 items-center gap-3">
								<span class="sf-meta-sm text-fg-dim w-16 text-right">
									{model.size_mb} MB
								</span>
								<div class="w-7 flex justify-end">
									{#if !model.downloaded && !modelStore.downloadingByModel[model.id]}
										<IconButton
											icon={Download}
											variant="normal"
											size="small"
											aria-label={`Install ${model.label}`}
											onclick={() => void download(model.id)}
										/>
									{/if}
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}

			<p class="sf-label-sm text-fg-dim px-0.5">
				You can manage and switch models any time in Settings.
			</p>
		</div>
	{/snippet}

	{#snippet footer()}
		<div></div>
		<div class="flex items-center gap-2">
			{#if hasAnyDownloaded}
				<Button variant="primary" onclick={selectAndContinue}>Continue</Button>
			{:else if !anyDownloading}
				<Button variant="ghost" onclick={onNext}>Skip for now</Button>
			{/if}
		</div>
	{/snippet}
</StepShell>
