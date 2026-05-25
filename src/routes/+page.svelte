<script lang="ts">
	import { browser } from "$app/environment";
	import { listen } from "@tauri-apps/api/event";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import ScribeScreen from "@lib/screens/scribe.svelte";
	import ScribeProcessingScreen from "@lib/screens/scribe-processing.svelte";
	import SettingsScreen from "@lib/screens/settings.svelte";
	import DictateScreen from "@lib/screens/dictate.svelte";
import TranscribeScreen from "@lib/screens/transcribe.svelte";

	const viewParam = browser ? new URLSearchParams(window.location.search).get("view") : null;
	const standaloneSettings = viewParam === "settings";
	const standaloneDictate = viewParam === "dictate";
const standaloneTranscribe = viewParam === "transcribe";

	type AppScreen = "recording" | "processing";

	let appScreen = $state<AppScreen>("recording");
	let processingTitle = $state("Recording");
	/** Kept inert per CLAUDE.md "Scribe recording auto-start" — opening Scribe lands at idle. */
	let autoStartRecording = $state(false);

	function beginProcessing(title: string) {
		processingTitle = title || "Recording";
		appScreen = "processing";
	}

	function returnToRecording() {
		appScreen = "recording";
	}

	async function closeStandaloneSettings() {
		await getCurrentWindow().close();
	}

	import { onMount } from "svelte";
	onMount(() => {
		if (standaloneSettings) return;
		void listen('scribe://open-requested', () => {
			appScreen = "recording";
		});
	});
</script>

{#if standaloneSettings}
	<SettingsScreen standalone onClose={closeStandaloneSettings} />
{:else if standaloneDictate}
	<DictateScreen />
{:else if standaloneTranscribe}
	<TranscribeScreen />
{:else}
	<main>
		{#if appScreen === "processing"}
			<ScribeProcessingScreen
				title={processingTitle}
				onRecordAgain={returnToRecording}
			/>
		{:else}
			<ScribeScreen processingStart={beginProcessing} bind:autoStart={autoStartRecording} />
		{/if}
	</main>
{/if}
