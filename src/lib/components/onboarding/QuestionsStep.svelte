<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { fade, fly } from "svelte/transition";
	import Button from "@lib/components/Button.svelte";
	import StepShell from "./StepShell.svelte";
	import { isWindows } from "$lib/platform";
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
		onNext: (updates: Partial<OnboardingAnswers>) => void;
	} = $props();

	type MainUse = "meetings" | "notes" | "files";
	let mainUse = $state<MainUse | null>(answers.mainUse);
	let preferAccuracy = $state<boolean | null>(answers.mainUse !== null ? answers.preferAccuracy : null);
	let speakerCapture = $state<boolean | null>(answers.mainUse !== null ? answers.speakerCapture : null);

	// Progressive reveal: unlock each question when the previous is answered
	const q2Visible = $derived(mainUse !== null);
	const q3Visible = $derived(q2Visible && preferAccuracy !== null);
	const canContinue = $derived(q3Visible && speakerCapture !== null);

	async function handleNext() {
		await invoke("settings_set_scribe_capture_speaker", { enabled: speakerCapture ?? false }).catch(() => {});
		onNext({
			mainUse: mainUse ?? "notes",
			preferAccuracy: preferAccuracy ?? false,
			speakerCapture: speakerCapture ?? false,
		});
	}

	const useOptions: { value: MainUse; label: string }[] = [
		{ value: "meetings", label: "Meetings & conversations" },
		{ value: "notes", label: "Quick voice notes" },
		{ value: "files", label: "Transcribe audio files" },
	];

	const accuracyOptions = [
		{ value: true, label: "Best accuracy" },
		{ value: false, label: "Fastest speed" },
	];
</script>

<StepShell {currentStep} title="Quick setup" subtitle="We'll pick the right settings for you — no technical decisions needed.">
	{#snippet children()}
		<div class="space-y-5">
			<!-- Q1: always visible -->
			<fieldset class="space-y-2">
				<legend class="font-mono text-label-sm tracking-stamped uppercase text-fg/80">What will you mainly use ScribeFloat for?</legend>
				<div class="space-y-1.5">
					{#each useOptions as opt}
						<label class="flex items-center gap-3 rounded-md bg-card border border-fill px-3 py-2.5 cursor-pointer hover:bg-fill transition-colors {mainUse === opt.value ? 'border-active' : ''}">
							<input type="radio" name="mainUse" value={opt.value} bind:group={mainUse} class="sr-only" />
							<span class="w-3.5 h-3.5 rounded-full border-2 shrink-0 {mainUse === opt.value ? 'border-active bg-active' : 'border-rim'}"></span>
							<span class="text-body-md text-fg">{opt.label}</span>
						</label>
					{/each}
				</div>
			</fieldset>

			<!-- Q2: appears after Q1 is answered -->
			{#if q2Visible}
				<div in:fly={{ y: 8, duration: 200 }} in:fade={{ duration: 150 }}>
					<fieldset class="space-y-2">
						<legend class="font-mono text-label-sm tracking-stamped uppercase text-fg/80">Transcription priority?</legend>
						<div class="flex gap-2">
							{#each accuracyOptions as opt}
								<label class="flex-1 flex items-center justify-center gap-2 rounded-md bg-card border border-fill px-3 py-2.5 cursor-pointer hover:bg-fill transition-colors {preferAccuracy === opt.value ? 'border-active bg-active/10' : ''}">
									<input type="radio" name="accuracy" checked={preferAccuracy === opt.value} onchange={() => { preferAccuracy = opt.value; }} class="sr-only" />
									<span class="text-body-md text-fg">{opt.label}</span>
								</label>
							{/each}
						</div>
						<p class="text-label-sm text-fg-dim px-1">
							{preferAccuracy
								? "Recommends a larger model (~460 MB). Slower but more accurate."
								: preferAccuracy === false
									? "Recommends a smaller model (~145 MB). Fast, good for quick notes."
									: ""}
						</p>
					</fieldset>
				</div>
			{/if}

			<!-- Q3: appears after Q2 is answered -->
			{#if q3Visible}
				<div in:fly={{ y: 8, duration: 200 }} in:fade={{ duration: 150 }}>
					<fieldset class="space-y-2">
						<legend class="font-mono text-label-sm tracking-stamped uppercase text-fg/80">Capture computer audio too?</legend>
						<div class="flex gap-2">
							{#each [{ value: true, label: "Mic + computer audio" }, { value: false, label: "Microphone only" }] as opt}
								<label class="flex-1 flex items-center justify-center gap-2 rounded-md bg-card border border-fill px-3 py-2.5 cursor-pointer hover:bg-fill transition-colors {speakerCapture === opt.value ? 'border-active bg-active/10' : ''}">
									<input type="radio" name="speaker" checked={speakerCapture === opt.value} onchange={() => { speakerCapture = opt.value; }} class="sr-only" />
									<span class="text-body-md text-fg">{opt.label}</span>
								</label>
							{/each}
						</div>
						{#if speakerCapture === true}
							<p class="text-label-sm text-fg-dim px-1">
								{isWindows
									? "Windows captures system audio automatically."
									: "macOS requires BlackHole (free virtual audio driver). We'll help you set it up."}
							</p>
						{/if}
					</fieldset>
				</div>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" disabled={!canContinue} onclick={handleNext}>Continue</Button>
	{/snippet}
</StepShell>
