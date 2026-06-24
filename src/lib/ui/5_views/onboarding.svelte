<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { onMount } from "svelte";
	import WelcomeStep from "@sections/onboarding/WelcomeStep.svelte";
	import ModelDownloadStep from "@sections/onboarding/ModelDownloadStep.svelte";
	import PermissionsStep from "@sections/onboarding/PermissionsStep.svelte";
	import DictatePracticeStep from "@sections/onboarding/DictatePracticeStep.svelte";
	import VoiceEnrollmentStep from "@sections/onboarding/VoiceEnrollmentStep.svelte";
	import FeatureTourStep from "@sections/onboarding/FeatureTourStep.svelte";
	import StepProgress from "@components/indicators/StepIndicator.svelte";
	import ScrollablePanel from "@primitives/layout/ScrollBody.svelte";
	import { appErrorMessage, type ModelListItem } from '@utils/types';

	// Steps: 1=Welcome, 2=ModelDownload, 3=Permissions, 4=DictatePractice, 5=FeatureTour
	let currentStep = $state(1);
	let skipModelStep = $state(false);
	let skipVoiceStep = $state(false);
	let error = $state("");
	const totalProgressSteps = $derived(skipVoiceStep ? 4 : 5);

	function next() {
		let n = currentStep + 1;
		if (n === 2 && skipModelStep) n = 3;
		if (n === 5 && skipVoiceStep) n = 6;
		currentStep = Math.min(n, 6);
	}

	function back() {
		let p = currentStep - 1;
		if (currentStep === 6 && skipVoiceStep) p = 4;
		if (p === 2 && skipModelStep) p = 1;
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
		const models = await invoke<ModelListItem[]>("model_list").catch(() => [] as ModelListItem[]);
		const profileNames = await invoke<string[]>("voiceprint_list_profile_names").catch(() => []);
		const anyDownloaded = models.some((m) => m.downloaded);
		skipModelStep = anyDownloaded;
		skipVoiceStep = profileNames.length > 0;

		// If a model is downloaded but none are selected, auto-select the first downloaded one.
		// This handles re-runs of onboarding (step 2 gets skipped) and fresh config after app update.
		if (anyDownloaded && !models.some((m) => m.selected)) {
			const first = models.find((m) => m.downloaded);
			if (first) {
				await invoke("model_select", { modelId: first.id }).catch((e: unknown) => {
					error = `Could not activate installed model: ${appErrorMessage(e)}`;
				});
			}
		}
	});
</script>

<div class="flex h-screen min-h-0 flex-col overflow-hidden bg-panel p-6">
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
			<ModelDownloadStep onNext={next} />
		{:else if currentStep === 3}
			<PermissionsStep onBack={back} onNext={next} />
		{:else if currentStep === 4}
			<DictatePracticeStep onBack={back} onNext={next} />
		{:else if currentStep === 5}
			<VoiceEnrollmentStep onBack={back} onNext={next} isFirstTime />
		{:else if currentStep === 6}
			<FeatureTourStep onBack={back} onFinish={finish} />
		{/if}
	</ScrollablePanel>
</div>
