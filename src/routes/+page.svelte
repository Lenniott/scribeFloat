<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import OnboardingScreen from "@lib/screens/onboarding.svelte";
	import ScribeScreen from "@lib/screens/scribe.svelte";

	let onboardingComplete = $state(false);
	let gateLoading = $state(true);
	let gateError = $state("");

	function getErrorMessage(error: unknown, fallback: string): string {
		if (typeof error === "string" && error.trim()) return error;
		if (error instanceof Error && error.message.trim()) return error.message;
		if (typeof error === "object" && error !== null) {
			const maybeMessage = (error as { message?: unknown }).message;
			if (typeof maybeMessage === "string" && maybeMessage.trim()) return maybeMessage;
		}
		return fallback;
	}

	async function refreshGate() {
		gateLoading = true;
		gateError = "";
		try {
			const [modelReady, permissions, outputPath, hotkeys] = await Promise.all([
				invoke<boolean>("model_setup_status"),
				invoke<Array<{ granted: boolean; can_request: boolean }>>("settings_permissions_status"),
				invoke<string>("settings_get_output_path"),
				invoke<[string, string]>("settings_get_hotkeys"),
			]);
			const [openHotkey, dictateHotkey] = hotkeys;
			const permissionsReady = permissions.every((p) => p.granted || !p.can_request);
			onboardingComplete =
				modelReady &&
				permissionsReady &&
				Boolean(outputPath.trim()) &&
				Boolean(openHotkey.trim() && dictateHotkey.trim());
		} catch (error) {
			onboardingComplete = false;
			gateError = getErrorMessage(error, "Could not verify onboarding status.");
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
