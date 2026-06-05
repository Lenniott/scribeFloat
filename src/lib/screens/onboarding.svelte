<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import WelcomeStep from "@lib/components/onboarding/WelcomeStep.svelte";
	import PermissionsStep from "@lib/components/onboarding/PermissionsStep.svelte";
	import QuestionsStep from "@lib/components/onboarding/QuestionsStep.svelte";
	import ModelStep from "@lib/components/onboarding/ModelStep.svelte";
	import ScribePracticeStep from "@lib/components/onboarding/ScribePracticeStep.svelte";
	import DictatePracticeStep from "@lib/components/onboarding/DictatePracticeStep.svelte";
	import HistoryStep from "@lib/components/onboarding/HistoryStep.svelte";
	import CompleteStep from "@lib/components/onboarding/CompleteStep.svelte";
	import type { OnboardingAnswers } from "$lib/types";

	let currentStep = $state(1);

	let answers = $state<OnboardingAnswers>({
		mainUse: null,
		preferAccuracy: false,
		speakerCapture: false,
		selectedModelId: null,
		saveMd: false,
	});

	function next() {
		currentStep = Math.min(currentStep + 1, 8);
	}

	function back() {
		currentStep = Math.max(currentStep - 1, 1);
	}

	function applyUpdates(updates: Partial<OnboardingAnswers>) {
		answers = { ...answers, ...updates };
		next();
	}

	async function finish() {
		await invoke("settings_complete_onboarding").catch(() => {});
		await getCurrentWindow().close();
	}

	async function skipToSettings() {
		await invoke("settings_complete_onboarding").catch(() => {});
		await invoke("settings_show_window").catch(() => {});
		await getCurrentWindow().close();
	}
</script>

<div class="flex flex-col items-center justify-center h-full p-6 gap-0 bg-panel">
	{#if currentStep === 1}
		<WelcomeStep onStart={next} onSkip={skipToSettings} />
	{:else if currentStep === 2}
		<PermissionsStep currentStep={2} onBack={back} onNext={next} />
	{:else if currentStep === 3}
		<QuestionsStep currentStep={3} {answers} onBack={back} onNext={applyUpdates} />
	{:else if currentStep === 4}
		<ModelStep currentStep={4} {answers} onBack={back} onNext={applyUpdates} />
	{:else if currentStep === 5}
		<ScribePracticeStep currentStep={5} {answers} onBack={back} onNext={next} />
	{:else if currentStep === 6}
		<DictatePracticeStep currentStep={6} onBack={back} onNext={next} />
	{:else if currentStep === 7}
		<HistoryStep currentStep={7} {answers} onBack={back} onNext={applyUpdates} />
	{:else if currentStep === 8}
		<CompleteStep currentStep={8} {answers} onFinish={finish} onOpenSettings={skipToSettings} />
	{/if}
</div>
