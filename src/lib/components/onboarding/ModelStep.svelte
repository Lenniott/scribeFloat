<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import Button from "@lib/components/Button.svelte";
	import StepShell from "./StepShell.svelte";
	import StackProgressBar from "@lib/components/form/StackProgressBar.svelte";
	import { CircleCheckBig } from "lucide-svelte";
	import type { ModelListItem, ModelProgressPayload, OnboardingAnswers } from "$lib/types";

	let {
		currentStep,
		answers,
		onBack,
		onNext,
	}: {
		currentStep: number;
		answers: OnboardingAnswers;
		onBack: () => void;
		onNext: (updates: Partial<OnboardingAnswers>) => void;
	} = $props();

	const MODEL_INFO: Record<string, { label: string; description: string; sizeMb: string }> = {
		"tiny-en-q5":   { label: "Tiny",   description: "Fastest — good for quick notes.",         sizeMb: "~75 MB"   },
		"base-en-q5":   { label: "Base",   description: "Fast with good accuracy. Great for most.", sizeMb: "~145 MB"  },
		"small-en-q5":  { label: "Small",  description: "More accurate, slightly slower.",          sizeMb: "~460 MB"  },
		"medium-en-q5": { label: "Medium", description: "Best accuracy. For complex audio.",         sizeMb: "~1.5 GB"  },
	};

	function deriveRecommendedId(a: OnboardingAnswers): string {
		if (a.preferAccuracy) return "small-en-q5";
		return "base-en-q5";
	}

	let models = $state<ModelListItem[]>([]);
	let recommendedId = $derived(deriveRecommendedId(answers));
	let selectedId = $state("base-en-q5");
	let showAll = $state(false);
	let progress = $state(0);
	let downloading = $state(false);
	let downloadError = $state("");
	let unlisteners: (() => void)[] = [];

	const recommended = $derived(models.find((m) => m.id === recommendedId));
	const selected = $derived(models.find((m) => m.id === selectedId));
	const alreadyInstalled = $derived(selected?.downloaded ?? false);
	const hasAnyInstalled = $derived(models.some((m) => m.downloaded));

	async function refresh() {
		models = await invoke<ModelListItem[]>("model_list").catch(() => []);
		if (models.some((m) => m.downloaded && m.selected)) return;
		const downloaded = models.filter((m) => m.downloaded);
		if (downloaded.length > 0) {
			await invoke("model_select", { modelId: downloaded[0].id }).catch(() => {});
			await refreshModels();
		}
	}

	async function refreshModels() {
		models = await invoke<ModelListItem[]>("model_list").catch(() => []);
	}

	async function startDownload() {
		downloadError = "";
		downloading = true;
		progress = 0;
		await invoke("model_download", { modelId: selectedId }).catch((e) => {
			downloadError = String(e);
			downloading = false;
		});
	}

	async function selectAndContinue() {
		await invoke("model_select", { modelId: selectedId }).catch(() => {});
		onNext({ selectedModelId: selectedId });
	}

	onMount(async () => {
		await refresh();
		if (recommended && !recommended.downloaded) selectedId = recommendedId;

		const ul1 = await listen<ModelProgressPayload>("model://download-progress", (e) => {
			if (e.payload.model_id !== selectedId) return;
			progress = Math.round(e.payload.progress * 100);
			if (e.payload.progress >= 1) {
				downloading = false;
				void refreshModels();
			}
		});
		const ul2 = await listen<string>("model://download-error", () => {
			downloading = false;
			downloadError = "Install failed. Check your connection and try again.";
		});
		unlisteners = [ul1, ul2];
	});

	onDestroy(() => unlisteners.forEach((u) => u()));
</script>

<StepShell {currentStep} title="Install a model" subtitle="Whisper runs entirely on your device. Download once, works offline forever.">
	{#snippet children()}
		<div class="space-y-3">
			{#if alreadyInstalled}
				<div class="flex items-center gap-3 rounded-md bg-card border border-fill px-3 py-3">
					<CircleCheckBig class="size-5 text-success shrink-0" />
					<div class="flex-1 min-w-0">
						<p class="text-body-md text-fg font-medium">{MODEL_INFO[selectedId]?.label ?? selectedId}</p>
						<p class="text-label-sm text-fg-dim">{MODEL_INFO[selectedId]?.description ?? ""}</p>
					</div>
					<span class="text-label-sm text-success">Installed</span>
				</div>
			{:else}
				<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-3">
					<div>
						<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Recommended for you</p>
						<p class="text-body-md text-fg font-medium mt-0.5">
							{MODEL_INFO[selectedId]?.label ?? selectedId}
							<span class="text-fg-dim font-normal ml-1">{MODEL_INFO[selectedId]?.sizeMb ?? ""}</span>
						</p>
						<p class="text-body-md text-fg-dim">{MODEL_INFO[selectedId]?.description ?? ""}</p>
					</div>

					{#if downloading}
						<StackProgressBar progress={progress} variant="small" />
					{:else if downloadError}
						<p class="text-label-sm text-destructive">{downloadError}</p>
					{/if}

					{#if !downloading}
						<Button variant="primary" onclick={startDownload}>
							Install model
						</Button>
					{/if}
				</div>
			{/if}

			{#if showAll}
				<div class="space-y-1.5">
					{#each models as m (m.id)}
						{#if m.id !== selectedId}
							<button
								type="button"
								class="w-full text-left flex items-center justify-between rounded-md bg-fill px-3 py-2 hover:bg-card transition-colors"
								onclick={() => { selectedId = m.id; showAll = false; }}
							>
								<span class="text-body-md text-fg">{MODEL_INFO[m.id]?.label ?? m.id}</span>
								<span class="text-label-sm text-fg-dim">{MODEL_INFO[m.id]?.sizeMb ?? ""}</span>
							</button>
						{/if}
					{/each}
				</div>
			{:else}
				<button type="button" class="text-label-sm text-fg-dim hover:text-fg transition-colors" onclick={() => { showAll = true; }}>
					Choose a different model
				</button>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<div class="flex items-center gap-2">
			{#if !alreadyInstalled && !downloading}
				<Button variant="ghost" onclick={() => onNext({ selectedModelId: null })}>Skip for now</Button>
			{/if}
			{#if alreadyInstalled}
				<Button variant="primary" onclick={selectAndContinue}>Continue</Button>
			{/if}
		</div>
	{/snippet}
</StepShell>
