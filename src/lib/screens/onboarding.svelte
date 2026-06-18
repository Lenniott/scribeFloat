<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { onMount } from "svelte";
	import WelcomeStep from "@lib/components/sections/onboarding/WelcomeStep.svelte";
	import ModelDownloadStep from "@lib/components/sections/onboarding/ModelDownloadStep.svelte";
	import PermissionsStep from "@lib/components/sections/onboarding/PermissionsStep.svelte";
	import DictatePracticeStep from "@lib/components/sections/onboarding/DictatePracticeStep.svelte";
	import FeatureTourStep from "@lib/components/sections/onboarding/FeatureTourStep.svelte";
	import StepProgress from "@lib/components/ui/indicators/StepIndicator.svelte";
	import ScrollablePanel from "@lib/components/primitives/layout/ScrollBody.svelte";
	import { appErrorMessage, type ModelListItem } from "$lib/types";

	// Steps: 1=Welcome, 2=ModelDownload, 3=Permissions, 4=DictatePractice, 5=FeatureTour
	let currentStep = $state(1);
	let skipModelStep = $state(false);
	let error = $state("");

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
		const anyDownloaded = models.some((m) => m.downloaded);
		skipModelStep = anyDownloaded;

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
			<StepProgress {currentStep} />
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
			<FeatureTourStep onBack={back} onFinish={finish} />
		{/if}
	</ScrollablePanel>
</div>
