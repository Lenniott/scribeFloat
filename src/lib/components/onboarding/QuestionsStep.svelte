<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
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
	let mainUse = $state<MainUse>(answers.mainUse ?? "notes");
	let preferAccuracy = $state(answers.preferAccuracy);
	let speakerCapture = $state(answers.speakerCapture);

	async function handleNext() {
		await invoke("settings_set_scribe_capture_speaker", { enabled: speakerCapture }).catch(() => {});
		onNext({ mainUse, preferAccuracy, speakerCapture });
	}

	const useOptions = [
		{ value: "meetings", label: "Meetings & conversations" },
		{ value: "notes", label: "Quick voice notes" },
		{ value: "files", label: "Transcribe audio files" },
	] as const;

	const accuracyOptions = [
		{ value: "accuracy", label: "Best accuracy" },
		{ value: "speed", label: "Fastest speed" },
	] as const;
</script>

<StepShell {currentStep} title="Quick setup" subtitle="We'll pick the right settings for you — no technical decisions needed.">
	{#snippet children()}
		<div class="space-y-6">
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

			<fieldset class="space-y-2">
				<legend class="font-mono text-label-sm tracking-stamped uppercase text-fg/80">Transcription priority?</legend>
				<div class="flex gap-2">
					{#each accuracyOptions as opt}
						<label class="flex-1 flex items-center justify-center gap-2 rounded-md bg-card border border-fill px-3 py-2.5 cursor-pointer hover:bg-fill transition-colors {(preferAccuracy ? 'accuracy' : 'speed') === opt.value ? 'border-active bg-active/10' : ''}">
							<input type="radio" name="accuracy" value={opt.value} checked={(preferAccuracy ? "accuracy" : "speed") === opt.value} onchange={() => { preferAccuracy = opt.value === "accuracy"; }} class="sr-only" />
							<span class="text-body-md text-fg">{opt.label}</span>
						</label>
					{/each}
				</div>
				<p class="text-label-sm text-fg-dim px-1">
					{preferAccuracy ? "Recommends a larger model (~460 MB). Slower but more accurate." : "Recommends a smaller model (~145 MB). Fast, good for quick notes."}
				</p>
			</fieldset>

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
				{#if speakerCapture}
					<p class="text-label-sm text-fg-dim px-1">
						{isWindows
							? "Windows captures system audio automatically."
							: "macOS requires BlackHole (free virtual audio driver). We'll help you set it up."}
					</p>
				{/if}
			</fieldset>
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={handleNext}>Continue</Button>
	{/snippet}
</StepShell>
