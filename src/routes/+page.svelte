<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import OnboardingScreen from "@lib/screens/onboarding.svelte";
	import ScribeScreen from "@lib/screens/scribe.svelte";
	import { extractErrorMessage } from "$lib/types";

	const forceOnboarding = import.meta.env.VITE_FORCE_ONBOARDING === "1";

	let onboardingComplete = $state(false);
	let gateLoading = $state(true);
	let gateError = $state("");

	async function refreshGate() {
		if (forceOnboarding) {
			onboardingComplete = false;
			gateError = "";
			gateLoading = false;
			return;
		}
		gateLoading = true;
		gateError = "";
		try {
			onboardingComplete = await invoke<boolean>("settings_onboarding_status");
		} catch (error) {
			onboardingComplete = false;
			gateError = extractErrorMessage(error, "Could not verify onboarding status.");
		} finally {
			gateLoading = false;
		}
	}

	onMount(refreshGate);
</script>

<main>
	{#if gateLoading}
		<div class="flex min-h-screen items-center justify-center p-6">
			<p class="text-body-sm text-on-surface/70">Loading app status...</p>
		</div>
	{:else if gateError}
		<div class="mx-auto flex min-h-screen w-full max-w-2xl flex-col items-center justify-center gap-3 p-6 text-center">
			<p class="text-title-sm font-semibold text-on-surface">Could not load app status</p>
			<p class="text-body-sm text-error">{gateError}</p>
			<Button variant="secondary" onclick={refreshGate}>Retry</Button>
		</div>
	{:else if onboardingComplete}
		<ScribeScreen />
	{:else}
		<OnboardingScreen onComplete={refreshGate} />
	{/if}
</main>
