<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import Button from "@lib/components/Button.svelte";
	import IconButton from "@lib/components/IconButton.svelte";
	import StepShell from "./StepShell.svelte";
	import { Download, CircleCheckBig } from "lucide-svelte";
	import type { ModelListItem, ModelProgressPayload } from "$lib/types";

	let { onNext }: { onNext: () => void } = $props();

	let models = $state<ModelListItem[]>([]);
	let progressByModel = $state<Record<string, number>>({});
	let downloadingByModel = $state<Record<string, boolean>>({});
	let error = $state("");
	let pollId: ReturnType<typeof setInterval> | undefined;
	let unlisteners: (() => void)[] = [];

	const hasAnyDownloaded = $derived(models.some((m) => m.downloaded));
	const anyDownloading = $derived(Object.values(downloadingByModel).some(Boolean));

	async function refresh() {
		const list = await invoke<ModelListItem[]>("model_list").catch(() => null);
		if (list) models = list;
	}

	async function download(modelId: string) {
		error = "";
		downloadingByModel = { ...downloadingByModel, [modelId]: true };
		progressByModel = { ...progressByModel, [modelId]: 0 };
		await invoke("model_download", { modelId }).catch((e: unknown) => {
			error = String(e);
			downloadingByModel = { ...downloadingByModel, [modelId]: false };
		});
	}

	async function selectAndContinue() {
		const downloaded = models.filter((m) => m.downloaded);
		const hasSelected = downloaded.some((m) => m.selected);
		if (downloaded.length > 0 && !hasSelected) {
			const err = await invoke("model_select", { modelId: downloaded[0].id }).catch(
				(e: unknown) => String(e),
			);
			if (typeof err === "string") {
				error = `Could not activate model: ${err}`;
				return;
			}
		}
		onNext();
	}

	onMount(async () => {
		const ul1 = await listen<ModelProgressPayload>("model://download-progress", (e) => {
			const { model_id, progress } = e.payload;
			progressByModel = { ...progressByModel, [model_id]: progress };
			if (progress >= 1) {
				downloadingByModel = { ...downloadingByModel, [model_id]: false };
				void refresh();
			}
		});
		const ul2 = await listen<string>("model://download-error", () => {
			for (const id of Object.keys(downloadingByModel)) {
				if (downloadingByModel[id]) {
					downloadingByModel = { ...downloadingByModel, [id]: false };
				}
			}
			error = "Install failed. Check your connection and try again.";
		});
		unlisteners = [ul1, ul2];

		await refresh();
		pollId = setInterval(() => void refresh(), 2000);
	});

	onDestroy(() => {
		unlisteners.forEach((u) => u());
		clearInterval(pollId);
	});

	function pct(modelId: string) {
		return Math.round((progressByModel[modelId] ?? 0) * 100);
	}
</script>

<StepShell
	title="Install AI model"
	subtitle="Whisper runs entirely on your device — nothing leaves your computer."
>
	{#snippet children()}
		<div class="space-y-3">
			{#if error}
				<p class="text-label-sm text-destructive px-1">{error}</p>
			{/if}

			{#if models.length === 0}
				<p class="text-label-sm text-fg-dim">Loading…</p>
			{:else}
				<div class="divide-y divide-fill overflow-hidden rounded-md border border-fill bg-panel">
					{#each models as model (model.id)}
						<div class="flex items-center gap-3 px-3 py-2.5">
							<div class="min-w-0 flex-1">
								<div class="flex items-baseline gap-2 flex-wrap">
									<span class={model.downloaded ? "text-body-md text-fg font-medium" : "text-body-md text-fg-dim"}>
										{model.label}
									</span>
									{#if downloadingByModel[model.id] || ((progressByModel[model.id] ?? 0) > 0 && !model.downloaded)}
										{@const p = pct(model.id)}
										<div class="flex items-center gap-2 flex-1 min-w-24">
											<div class="h-0.5 flex-1 overflow-hidden rounded-sm bg-fill">
												<div
													class="h-full rounded-sm bg-brand transition-[width] duration-200"
													style={`width:${p}%`}
												></div>
											</div>
											<span class="font-mono text-label-sm text-fg-dim tabular-nums">
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
								<span class="font-mono text-label-sm text-fg-dim tabular-nums w-16 text-right">
									{model.size_mb} MB
								</span>
								<div class="w-7 flex justify-end">
									{#if !model.downloaded && !downloadingByModel[model.id]}
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

			<p class="text-label-sm text-fg-dim px-0.5">
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
