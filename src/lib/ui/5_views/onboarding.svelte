<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { onMount } from "svelte";
	import WelcomeStep from "@sections/onboarding/WelcomeStep.svelte";
	import PermissionsStep from "@sections/onboarding/PermissionsStep.svelte";
	import DictatePracticeStep from "@sections/onboarding/DictatePracticeStep.svelte";
	import VoiceEnrollmentStep from "@sections/onboarding/VoiceEnrollmentStep.svelte";
	import FeatureTourStep from "@sections/onboarding/FeatureTourStep.svelte";
	import StepProgress from "@components/indicators/StepIndicator.svelte";
	import ScrollablePanel from "@primitives/layout/ScrollBody.svelte";
	import { appErrorMessage } from '@utils/types';

	// Steps: 1=Welcome, 2=Permissions, 3=DictatePractice, 4=VoiceEnrollment, 5=FeatureTour
	let currentStep = $state(1);
	let skipVoiceStep = $state(false);
	let error = $state("");
	const totalProgressSteps = $derived(skipVoiceStep ? 3 : 4);
	// Keep the progress dots contiguous when the voice step is skipped.
	const indicatorStep = $derived(
		skipVoiceStep && currentStep > 4 ? currentStep - 1 : currentStep,
	);

	function next() {
		let n = currentStep + 1;
		if (n === 4 && skipVoiceStep) n = 5;
		currentStep = Math.min(n, 5);
	}

	function back() {
		let p = currentStep - 1;
		if (p === 4 && skipVoiceStep) p = 3;
		currentStep = Math.max(p, 1);
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
		const profileNames = await invoke<string[]>("voiceprint_list_profile_names").catch(() => []);
		skipVoiceStep = profileNames.length > 0;
	});
</script>

<div class="flex h-screen min-h-0 flex-col overflow-hidden bg-panel p-6">
	<!-- Progress indicator: single persistent instance so CSS transitions fire -->
	{#if currentStep > 1}
		<div class="mb-5 flex w-full shrink-0 justify-center">
			<StepProgress currentStep={indicatorStep} totalSteps={totalProgressSteps} />
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
			<VoiceEnrollmentStep onBack={back} onNext={next} isFirstTime />
		{:else if currentStep === 5}
			<FeatureTourStep onBack={back} onFinish={finish} />
		{/if}
	</ScrollablePanel>
</div>
