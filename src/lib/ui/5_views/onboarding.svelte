<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { onMount } from "svelte";
	import WelcomeStep from "@sections/onboarding/WelcomeStep.svelte";
	import PermissionsStep from "@sections/onboarding/PermissionsStep.svelte";
	import DictatePracticeStep from "@sections/onboarding/DictatePracticeStep.svelte";
	import FeatureTourStep from "@sections/onboarding/FeatureTourStep.svelte";
	import StepProgress from "@components/indicators/StepIndicator.svelte";
	import ScrollablePanel from "@primitives/layout/ScrollBody.svelte";
	import { appErrorMessage } from '@utils/types';

	// Steps: 1=Welcome, 2=Permissions, 3=DictatePractice, 4=FeatureTour
	let currentStep = $state(1);
	let ready = $state(false);
	let error = $state("");
	const totalProgressSteps = 3;

	function clampStep(step: number): number {
		return Math.min(4, Math.max(1, Math.trunc(step) || 1));
	}

	async function persistStep(step: number) {
		const next = clampStep(step);
		currentStep = next;
		await invoke("settings_set_onboarding_step", { step: next }).catch(() => {});
	}

	function next() {
		void persistStep(currentStep + 1);
	}

	function back() {
		void persistStep(currentStep - 1);
	}

	async function finish() {
		error = "";
		try {
			await invoke("settings_complete_onboarding");
			await getCurrentWindow().close();
		} catch (e) {
			error = `Could not finish setup: ${appErrorMessage(e)}`;
		}
	}

	async function skipToSettings() {
		error = "";
		try {
			await invoke("settings_complete_onboarding");
			await invoke("settings_show_window");
			await getCurrentWindow().close();
		} catch (e) {
			error = `Could not open Settings: ${appErrorMessage(e)}`;
		}
	}

	onMount(async () => {
		const saved = await invoke<number>("settings_get_onboarding_step").catch(() => 1);
		currentStep = clampStep(saved);
		ready = true;
	});
</script>

<div class="flex h-screen min-h-0 flex-col overflow-hidden bg-panel p-6">
	{#if ready}
		<!-- Progress indicator: single persistent instance so CSS transitions fire -->
		{#if currentStep > 1}
			<div class="mb-5 flex w-full shrink-0 justify-center">
				<StepProgress {currentStep} totalSteps={totalProgressSteps} />
			</div>
		{/if}

		<ScrollablePanel class="flex flex-col">
			{#if error}
				<p class="mb-3 shrink-0 rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
					{error}
				</p>
			{/if}

			{#if currentStep === 1}
				<WelcomeStep onStart={next} onSkip={skipToSettings} />
			{:else if currentStep === 2}
				<PermissionsStep onBack={back} onNext={next} />
			{:else if currentStep === 3}
				<DictatePracticeStep onBack={back} onNext={next} />
			{:else if currentStep === 4}
				<FeatureTourStep onBack={back} onFinish={finish} />
			{/if}
		</ScrollablePanel>
	{/if}
</div>
