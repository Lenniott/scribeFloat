<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import StepShell from "./StepShell.svelte";
	import type { OnboardingAnswers } from "$lib/types";

	let {
		currentStep,
		answers,
		onBack,
		onNext,
	}: {
		currentStep: number;
		answers: OnboardingAnswers;
		onBack: () => void;
		onNext: () => void;
	} = $props();

	let savePath = $state<string>("");

	onMount(async () => {
		savePath = await invoke<string>("settings_get_output_path").catch(() => "");
	});

	const USE_LABELS: Record<string, string> = {
		meetings: "Meetings & conversations",
		notes: "Quick voice notes",
		files: "Transcribing audio files",
	};

	const modelLabel = $derived(answers.preferAccuracy ? "Small (accurate)" : "Base (fast)");
	const useLabel = $derived(answers.mainUse ? USE_LABELS[answers.mainUse] : "Quick voice notes");
</script>

<StepShell {currentStep} title="Here's what we've set up for you" subtitle="Based on your answers. You can adjust any of these in Settings later.">
	{#snippet children()}
		<div class="rounded-md bg-card border border-fill divide-y divide-fill">
			<div class="flex items-center justify-between px-3 py-2.5 gap-3">
				<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Best for</span>
				<span class="text-body-md text-fg">{useLabel}</span>
			</div>
			<div class="flex items-center justify-between px-3 py-2.5 gap-3">
				<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Model</span>
				<span class="text-body-md text-fg">{modelLabel}</span>
			</div>
			<div class="flex items-center justify-between px-3 py-2.5 gap-3">
				<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Speaker capture</span>
				<span class="text-body-md text-fg">{answers.speakerCapture ? "On — mic + computer audio" : "Off — mic only"}</span>
			</div>
			{#if savePath}
				<div class="flex items-start justify-between px-3 py-2.5 gap-3">
					<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70 shrink-0">Saves to</span>
					<span class="text-body-md text-fg text-right break-all">{savePath}</span>
				</div>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Change answers</Button>
		<Button variant="primary" onclick={onNext}>Looks good — install model</Button>
	{/snippet}
</StepShell>
