<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import OnboardingScreen from "@lib/screens/onboarding.svelte";
	import ScribeScreen from "@lib/screens/scribe.svelte";

	let onboardingComplete = $state(false);

	async function refreshGate() {
		const modelReady = await invoke<boolean>("model_setup_status").catch(() => false);
		const permissions = await invoke<Array<{ granted: boolean; can_request: boolean }>>(
			"settings_permissions_status",
		).catch(() => []);
		const outputPath = await invoke<string>("settings_get_output_path").catch(() => "");
		const [openHotkey, dictateHotkey] = await invoke<[string, string]>("settings_get_hotkeys").catch(
			() => ["", ""],
		);
		const permissionsReady = permissions.every((p) => p.granted || !p.can_request);
		onboardingComplete =
			modelReady &&
			permissionsReady &&
			Boolean(outputPath.trim()) &&
			Boolean(openHotkey.trim() && dictateHotkey.trim());
	}

	onMount(refreshGate);
</script>

<main>
	{#if onboardingComplete}
		<ScribeScreen />
	{:else}
		<OnboardingScreen onComplete={refreshGate} />
	{/if}
</main>
