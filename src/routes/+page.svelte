<script lang="ts">
	import { onMount } from "svelte";
	import { browser } from "$app/environment";
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import Button from "@lib/components/Button.svelte";
	import OnboardingScreen from "@lib/screens/onboarding.svelte";
	import ScribeScreen from "@lib/screens/scribe.svelte";
	import ScribeProcessingScreen from "@lib/screens/scribe-processing.svelte";
	import SettingsScreen from "@lib/screens/settings.svelte";
	import { extractErrorMessage } from "$lib/types";

	const forceOnboarding = import.meta.env.VITE_FORCE_ONBOARDING === "1";
	const skipOnboarding = import.meta.env.VITE_SKIP_ONBOARDING === "1" && !forceOnboarding;
	const standaloneSettings = browser && new URLSearchParams(window.location.search).get("view") === "settings";

	type AppScreen = "recording" | "processing";

	let onboardingComplete = $state(false);
	let gateLoading = $state(true);
	let gateError = $state("");
	let appScreen = $state<AppScreen>("recording");
	let processingTitle = $state("Recording");
	let autoStartRecording = $state(true);

	async function refreshGate() {
		if (skipOnboarding) {
			onboardingComplete = true;
			gateError = "";
			gateLoading = false;
			appScreen = "recording";
			return;
		}

		if (forceOnboarding) {
			onboardingComplete = false;
			gateError = "";
			gateLoading = false;
			appScreen = "recording";
			return;
		}
		gateLoading = true;
		gateError = "";
		try {
			onboardingComplete = await invoke<boolean>("settings_onboarding_status");
			appScreen = "recording";
		} catch (error) {
			onboardingComplete = false;
			gateError = extractErrorMessage(error, "Could not verify onboarding status.");
		} finally {
			gateLoading = false;
		}
	}

	function beginProcessing(title: string) {
		processingTitle = title || "Recording";
		appScreen = "processing";
	}

	function returnToRecording() {
		autoStartRecording = true;
		appScreen = "recording";
	}

	async function closeProcessing() {
		autoStartRecording = false;
		await getCurrentWindow().close().catch(() => {
			appScreen = "recording";
		});
	}

	async function closeStandaloneSettings() {
		await getCurrentWindow().close();
	}

	function finishOnboarding() {
		onboardingComplete = true;
		gateError = "";
		gateLoading = false;
		appScreen = "recording";
	}

	onMount(() => {
		if (!standaloneSettings) void refreshGate();
	});
</script>

{#if standaloneSettings}
	<SettingsScreen standalone onClose={closeStandaloneSettings} />
{:else}
	<main>
		{#if gateLoading}
			<div class="flex min-h-screen items-center justify-center p-6">
				<p class="text-body-sm text-on-surface/70">Loading app status...</p>
			</div>
		{:else if gateError}
			<div class="mx-auto flex min-h-screen w-full max-w-2xl flex-col items-center justify-center gap-3 p-6 text-center">
				<p class="text-title-sm font-normal tracking-tight text-on-surface">Could not load app status</p>
				<p class="text-body-sm text-error">{gateError}</p>
				<Button variant="secondary" onclick={refreshGate}>Retry</Button>
			</div>
		{:else if onboardingComplete}
			{#if appScreen === "processing"}
				<ScribeProcessingScreen title={processingTitle} onClose={closeProcessing} onRecordAgain={returnToRecording} />
			{:else}
				<ScribeScreen processingStart={beginProcessing} autoStart={autoStartRecording} />
			{/if}
		{:else}
			<OnboardingScreen onComplete={finishOnboarding} />
		{/if}
	</main>
{/if}
