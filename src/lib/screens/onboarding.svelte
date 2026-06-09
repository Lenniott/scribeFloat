<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { onMount } from "svelte";
	import WelcomeStep from "@lib/components/onboarding/WelcomeStep.svelte";
	import ModelDownloadStep from "@lib/components/onboarding/ModelDownloadStep.svelte";
	import PermissionsStep from "@lib/components/onboarding/PermissionsStep.svelte";
	import DictatePracticeStep from "@lib/components/onboarding/DictatePracticeStep.svelte";
	import FeatureTourStep from "@lib/components/onboarding/FeatureTourStep.svelte";
	import StepProgress from "@lib/components/onboarding/StepProgress.svelte";
	import type { ModelListItem } from "$lib/types";

	// Steps: 1=Welcome, 2=ModelDownload, 3=Permissions, 4=DictatePractice, 5=FeatureTour
	let currentStep = $state(1);
	let skipModelStep = $state(false);

	function next() {
		let n = currentStep + 1;
		if (n === 2 && skipModelStep) n = 3;
		currentStep = Math.min(n, 5);
	}

	function back() {
		let p = currentStep - 1;
		if (p === 2 && skipModelStep) p = 1;
		currentStep = Math.max(p, 1);
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

	onMount(async () => {
		const models = await invoke<ModelListItem[]>("model_list").catch(() => [] as ModelListItem[]);
		skipModelStep = models.some((m) => m.downloaded);
	});
</script>

<div class="flex flex-col h-screen overflow-hidden p-6 bg-panel">
	<!-- Progress indicator: single persistent instance so CSS transitions fire -->
	{#if currentStep > 1}
		<div class="w-full flex justify-center mb-5 shrink-0">
			<StepProgress {currentStep} />
		</div>
	{/if}

	<div class="flex-1 min-h-0">
		{#if currentStep === 1}
			<WelcomeStep onStart={next} onSkip={skipToSettings} />
		{:else if currentStep === 2}
			<ModelDownloadStep onNext={next} />
		{:else if currentStep === 3}
			<PermissionsStep onBack={back} onNext={next} />
		{:else if currentStep === 4}
			<DictatePracticeStep onBack={back} onNext={next} />
		{:else if currentStep === 5}
			<FeatureTourStep onBack={back} onFinish={finish} />
		{/if}
	</div>
</div>
