<script lang="ts">
	import { browser } from "$app/environment";
	import { listen } from "@tauri-apps/api/event";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import ScribeScreen from "@lib/screens/scribe.svelte";
	import ScribeProcessingScreen from "@lib/screens/scribe-processing.svelte";
	import SettingsScreen from "@lib/screens/settings.svelte";

	const standaloneSettings = browser && new URLSearchParams(window.location.search).get("view") === "settings";

	type AppScreen = "recording" | "processing";

	let appScreen = $state<AppScreen>("recording");
	let processingTitle = $state("Recording");
	let autoStartRecording = $state(true);

	function beginProcessing(title: string) {
		processingTitle = title || "Recording";
		appScreen = "processing";
	}

	function returnToRecording() {
		autoStartRecording = true;
		appScreen = "recording";
	}

	async function closeStandaloneSettings() {
		await getCurrentWindow().close();
	}

	import { onMount } from "svelte";
	onMount(() => {
		if (standaloneSettings) return;
		void listen('scribe://open-requested', () => {
			autoStartRecording = true;
			appScreen = "recording";
		});
	});
</script>

{#if standaloneSettings}
	<SettingsScreen standalone onClose={closeStandaloneSettings} />
{:else}
	<main>
		{#if appScreen === "processing"}
			<ScribeProcessingScreen
				title={processingTitle}
				onRecordAgain={returnToRecording}
			/>
		{:else}
			<ScribeScreen processingStart={beginProcessing} autoStart={autoStartRecording} />
		{/if}
	</main>
{/if}
