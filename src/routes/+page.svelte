<script lang="ts">
	import { browser } from "$app/environment";
	import ScribeScreen from "@lib/screens/scribe.svelte";
	import ScribeProcessingScreen from "@lib/screens/scribe-processing.svelte";
	import SettingsScreen from "@lib/screens/settings.svelte";
	import DictateScreen from "@lib/screens/dictate.svelte";
	import HistoryScreen from "@lib/screens/history.svelte";
	import TranscribeScreen from "@lib/screens/transcribe.svelte";

	const viewParam = browser ? new URLSearchParams(window.location.search).get("view") : null;
	const standaloneSettings = viewParam === "settings";
	const standaloneDictate = viewParam === "dictate";
	const standaloneHistory = viewParam === "history";
	const standaloneTranscribe = viewParam === "transcribe";

	type AppScreen = "recording" | "processing";

	let appScreen = $state<AppScreen>("recording");
	let processingTitle = $state("Recording");

	function beginProcessing(title: string) {
		processingTitle = title || "Recording";
		appScreen = "processing";
	}

	function returnToRecording() {
		appScreen = "recording";
	}
</script>

{#if standaloneSettings}
	<SettingsScreen standalone />
{:else if standaloneDictate}
	<DictateScreen />
{:else if standaloneHistory}
	<HistoryScreen />
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
			<ScribeScreen processingStart={beginProcessing} />
		{/if}
	</main>
{/if}
